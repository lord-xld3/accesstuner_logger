use std::{
    fs::File,
    io,
};
use csv::Writer;

/// Writes the provided x and y data arrays to a CSV file with the given filename.
///
/// The function iterates over the paired x and y data, writes each pair as a record 
/// to the CSV, and then flushes the writer to ensure all data is written.
///
/// # Arguments
///
/// * `filename` - A string slice that holds the name of the file (with path if needed) to create or overwrite.
/// * `x_data` - A slice of floating-point numbers representing the x-axis data.
/// * `y_data` - A slice of floating-point numbers representing the y-axis data.
///
/// # Returns
///
/// * `Ok(())` if the data is successfully written to the CSV.
/// * `Err(e)` where `e` is the error produced while writing to the file.
///
/// # Examples
///
/// ```
/// let x_data = vec![1.0, 2.0, 3.0];
/// let y_data = vec![2.0, 4.0, 6.0];
/// write_to_csv("output.csv", &x_data, &y_data);
/// ```
pub fn write_to_csv(filename: &str, x_data: &[f32], y_data: &[f32]) -> io::Result<()> {
    let mut wtr = Writer::from_writer(File::create(filename)?);
    for (x_val, y_val) in x_data.iter().zip(y_data.iter()) {
        wtr.write_record(&[x_val.to_string(), y_val.to_string()])?;
    }
    wtr.flush()?;
    Ok(())
}
