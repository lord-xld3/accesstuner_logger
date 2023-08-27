// Import necessary modules and libraries
mod data;
mod csv_out;
mod expo_curve;

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::Path,
};
use data::{F32, LogData, LogField};
use csv_out::write_to_csv;
use expo_curve::run;
use time::Instant;

/// Main function for the program.
///
/// This function processes the OBD2 CSV log by:
/// 1. Reading the CSV file and extracting its headers.
/// 2. Verifying that all required headers are present.
/// 3. Parsing the CSV file line-by-line and extracting relevant data.
/// 4. Combining and correcting the extracted data.
/// 5. Deduplicating the X and Y values for curve fitting.
/// 6. Exporting the pre-corrected and post-corrected data to separate CSV files.
#[tokio::main]
async fn main() -> io::Result<()> {
    let start = Instant::now();
    let mut deduplicated_x = Vec::new();
    let mut deduplicated_y = Vec::new();
    // Check if stock.csv exists
    if Path::new("./data/stock.csv").exists() {
        if let Ok(lines) = read_lines("./data/stock.csv") {
            for line in lines {
                if let Ok(record) = line {
                    let values: Vec<&str> = record.split(',').collect();
                    deduplicated_x.push(values[0].parse().unwrap());
                    deduplicated_y.push(values[1].parse().unwrap());
                }
            }
        }
    } else {
        let log = fs::File::open("./data/log1.csv").map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                io::Error::new(e.kind(), "CSV log file not found. Ensure the path is correct and the file exists.")
            } else {
                e
            }
        })?;
        
        let reader = BufReader::new(log);
        let mut lines = reader.lines();

        // Extract the headers from the first line of the CSV
        let headers_line = lines.next().unwrap()?;
        let headers: Vec<&str> = headers_line.split(',').collect();

        // Create a mapping from headers to their corresponding column indices
        let mut indices = HashMap::new();
        for (i, header) in headers.iter().enumerate() {
            if let Some(field) = LogField::variants().iter().find(|&&field| header.contains(field.to_header())) {
                indices.insert(field.to_header(), i);
            }
        }

        // Ensure all required headers (defined by LogField variants) are present in the CSV
        let required_headers: Vec<&str> = LogField::variants().iter().map(|variant| variant.to_header()).collect();
        let missing_headers: Vec<&str> = required_headers.iter()
            .filter(|&&key| !indices.contains_key(key))
            .cloned()
            .collect();

        if !missing_headers.is_empty() {
            let missing_list = missing_headers.join(", ");
            panic!("The following headers were not found: {}", missing_list);
        }


        // Initialize a structure to hold the extracted log data
        let mut log_data = LogData::default();

        // Use a HashSet to ensure unique key-value combinations
        let mut seen = HashSet::new();

        // Process each line in the CSV, extracting and organizing relevant data
        for line in lines {
            let line = line?;
            let columns: Vec<&str> = line.split(',').collect();

            for &key in required_headers.iter() {
                if let Some(&index) = indices.get(key) {
                    if let Ok(value) = columns[index].parse::<f32>() {
                        if let Some(field) = LogField::from_header(key) {
                            log_data.push(field, value, &mut seen);
                        }
                    }
                }
            }
        }

        // Deduplicate X and Y values in preparation for curve fitting
        let mut seen_xy = HashSet::new();

        // Combine STFT and LTFT values to compute the combined fuel trim correction factor
        let ft_combine: Vec<f32> = log_data.get(&LogField::STFT).unwrap().iter().zip(log_data.get(&LogField::LTFT).unwrap())
        .map(|(&stft, &ltft)| stft + ltft)
        .collect();

        // Correct the MAF data using the combined fuel trim values
        let maf_cor: Vec<f32> = ft_combine.iter().zip(log_data.get(&LogField::MASS).unwrap())
        .map(|(&ft, &maf)| ft + maf)
        .collect();

        // Deduplicate data in preparation for curve fitting
        for (x_val, y_val) in log_data.get(&LogField::MAFV).unwrap().iter().zip(maf_cor.iter()) {
            let unique_key = (F32(*x_val), F32(*y_val));
            if !seen_xy.contains(&unique_key) {
                seen_xy.insert(unique_key);
                deduplicated_x.push(*x_val);
                deduplicated_y.push(*y_val);
            }
        }
    }
    // Export the deduplicated data for further analysis
    write_to_csv("pre-correction.csv", &deduplicated_x, &deduplicated_y)?;

    // Call the run function to get the corrected y data
    println!("Starting curve fitting");
    let (best_a, best_n) = run(&deduplicated_x, &deduplicated_y).await?;
    // Compute the fitted y values using the optimized parameters
    let y_fit: Vec<f32> = deduplicated_x.iter()
        .map(|&x| best_a * x.powf(best_n))
        .collect();

    // Export the fitted data for comparison
    write_to_csv("post-correction.csv", &deduplicated_x, &y_fit)?;
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);
    Ok(())
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}