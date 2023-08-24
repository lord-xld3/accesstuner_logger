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

/// Main function responsible for processing the OBD2 CSV log, 
/// extracting relevant data, performing curve fitting, and exporting 
/// the pre-corrected and post-corrected data to separate CSV files.
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

    // Map headers to their respective column indices
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

    // Ensure all required headers are found in the CSV
    let required_headers: Vec<&str> = LogField::variants().iter().map(|variant| variant.to_header()).collect();
    let missing_headers: Vec<&str> = required_headers.iter()
        .filter(|&&key| !indices.contains_key(key))
        .cloned()
        .collect();

    if !missing_headers.is_empty() {
        let missing_list = missing_headers.join(", ");
        panic!("The following headers were not found: {}", missing_list);
    }

    // Initialize the log data structure to hold extracted data from the CSV
    let mut log_data = LogData::default();

    // HashSet to ensure unique combinations of key and value are added to log_data
    let mut seen = HashSet::new();

    // Process each line in the CSV, extracting relevant data
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


    // Prepare for deduplication of X and Y values
    let mut seen_xy = HashSet::new();
    let mut deduplicated_x = Vec::new();
    let mut deduplicated_y = Vec::new();

    // Combine STFT and LTFT values to get combined fuel trim correction factor
    let ft_combine: Vec<f32> = log_data.get(&LogField::STFT).unwrap().iter().zip(log_data.get(&LogField::LTFT).unwrap())
    .map(|(&stft, &ltft)| stft + ltft)
    .collect();

    // Apply fuel trim correction factors to the MAF data
    let maf_cor: Vec<f32> = ft_combine.iter().zip(log_data.get(&LogField::MASS).unwrap())
    .map(|(&ft, &maf)| ft + maf)
    .collect();

    // Deduplicate X and Y values before curve fitting
    for (x_val, y_val) in log_data.get(&LogField::MAFV).unwrap().iter().zip(maf_cor.iter()) {
        let unique_key = (F32(*x_val), F32(*y_val));
        if !seen_xy.contains(&unique_key) {
            seen_xy.insert(unique_key);
            deduplicated_x.push(*x_val);
            deduplicated_y.push(*y_val);
        }
    }

    // Export deduplicated data to "pre-correction.csv"
    write_to_csv("pre-correction.csv", &deduplicated_x, &deduplicated_y)?;

    // Placeholder for curve fitting logic
    // let (a_opt, b_opt, c_opt) = curve_fit(&deduplicated_x, &deduplicated_y);

    // Sample optimized parameters
    let a_opt = 1.0;
    let b_opt = 1.0;
    let c_opt = 1.0;

    // Calculate the corresponding y values using the sample optimized parameters
    let y_fit: Vec<f32> = deduplicated_x.iter().map(|&x_val| a_opt * (-b_opt * x_val).exp() + c_opt).collect();

    // Export fitted data to "post-correction.csv"
    write_to_csv("post-correction.csv", &deduplicated_x, &y_fit)?;
    Ok(())
}

/// Performs curve fitting on the provided x_data and y_data.
///
/// # Arguments
/// - x_data: A slice of f32 values representing the x data.
/// - y_data: A slice of f32 values representing the y data.
///
/// # Returns
/// A tuple containing three f32 values representing the optimized parameters for the curve.
fn curve_fit(x_data: &[f32], y_data: &[f32]) -> (f32, f32, f32) {
    // TODO: Implement the curve fitting logic here.
    // For now, we'll just return dummy values.
    (1.0, 1.0, 1.0)
}
