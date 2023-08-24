// Import necessary modules and libraries
mod data;
mod csv_out;

use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, BufRead, BufReader},
};
use data::{F32, LogData, LogField};
use csv_out::write_to_csv;

/// Main function for the program.
///
/// This function processes the OBD2 CSV log by:
/// 1. Reading the CSV file and extracting its headers.
/// 2. Verifying that all required headers are present.
/// 3. Parsing the CSV file line-by-line and extracting relevant data.
/// 4. Combining and correcting the extracted data.
/// 5. Deduplicating the X and Y values for curve fitting.
/// 6. Exporting the pre-corrected and post-corrected data to separate CSV files.
fn main() -> io::Result<()> {
    // Open and read the CSV file
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
        match *header {
            "MAF Voltage (V)" => { indices.insert("MAFV", i); },
            "Mass Airflow (g/s)" => { indices.insert("MASS", i); },
            "Short Term FT (%)" => { indices.insert("STFT", i); },
            "Long Term FT (%)" => { indices.insert("LTFT", i); },
            _ => {}
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
    let mut deduplicated_x = Vec::new();
    let mut deduplicated_y = Vec::new();

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

    // Export the deduplicated data for further analysis
    write_to_csv("pre-correction.csv", &deduplicated_x, &deduplicated_y)?;

    // Placeholder logic for curve fitting - this will be replaced with actual curve fitting logic in future iterations
    let a_opt = 1.0;
    let b_opt = 1.0;
    let c_opt = 1.0;

    // Compute the Y values using the curve fitting parameters
    let y_fit: Vec<f32> = deduplicated_x.iter().map(|&x_val| a_opt * (-b_opt * x_val).exp() + c_opt).collect();

    // Export the fitted data for comparison
    write_to_csv("post-correction.csv", &deduplicated_x, &y_fit)?;
    Ok(())
}

/// A placeholder function for curve fitting.
///
/// # Arguments
/// * `x_data`: A slice of `f32` values representing the x data points.
/// * `y_data`: A slice of `f32` values representing the y data points.
///
/// # Returns
/// A tuple of three `f32` values representing the optimized parameters of the curve.
///
/// # Note
/// This is a stub and needs to be implemented with actual curve fitting logic.
fn curve_fit(x_data: &[f32], y_data: &[f32]) -> (f32, f32, f32) {
    (1.0, 1.0, 1.0)
}
