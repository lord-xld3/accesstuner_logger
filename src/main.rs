use std::fs;

fn main() {
    let log = fs::read_to_string("./data/log1.csv")
        .expect("Should be able to read log file");
    let headerindex = log.find("\r\n");
    
    let headers:&str = match headerindex {
        Some(index) => &log[0..index],
        None => panic!("Cannot read headers row!"),
    };
    
    let column_headers: Vec<String> = headers.split(',').map(|s| s.to_string()).collect();
    let mut mafvindex: Option<usize> = None;
    let mut massindex: Option<usize> = None;
    let mut shortindex: Option<usize> = None;
    let mut longindex: Option<usize> = None;

    for (i, e) in column_headers.into_iter().enumerate() {
        match e.as_str() {
            "MAF Voltage (V)" => {
                mafvindex = Some(i);
            }
            "Mass Airflow (g/s)" => {
                massindex = Some(i);
            }
            "Short Term FT (%)" => {
                shortindex = Some(i);
            }
            "Long Term FT (%)" => {
                longindex = Some(i);
            }
            _ => {}
        }
    }

    if mafvindex.is_some() && massindex.is_some() && shortindex.is_some() && longindex.is_some() {
        println!("All matches were successful!");
        println!("MAF V index: {:?}", mafvindex.unwrap());
        println!("Mass A index: {:?}", massindex.unwrap());
        println!("Short T index: {:?}", shortindex.unwrap());
        println!("Long T index: {:?}", longindex.unwrap());
    } else {
        println!("The following matches were not successful:");
        if mafvindex.is_none() {
            println!("- MAF V");
        }
        if massindex.is_none() {
            println!("- Mass A");
        }
        if shortindex.is_none() {
            println!("- Short T");
        }
        if longindex.is_none() {
            println!("- Long T");
        }
    }

    let mut mafv_values: Vec<f32> = Vec::new();
    let mut mass_values: Vec<f32> = Vec::new();
    let mut short_values: Vec<f32> = Vec::new();
    let mut long_values: Vec<f32> = Vec::new();
    
    for line in log.lines().skip(1) {
        let columns: Vec<&str> = line.split(',').collect();
    
        // MAF V
        if let Some(mafv_value) = columns.get(mafvindex.unwrap()) {
            if let Ok(value) = mafv_value.parse::<f32>() {
                mafv_values.push(value);
            }
        }
    
        // Mass A
        if let Some(mass_value) = columns.get(massindex.unwrap()) {
            if let Ok(value) = mass_value.parse::<f32>() {
                mass_values.push(value);
            }
        }
    
        // Short T
        if let Some(short_value) = columns.get(shortindex.unwrap()) {
            if let Ok(value) = short_value.parse::<f32>() {
                short_values.push(value);
            }
        }
    
        // Long T
        if let Some(long_value) = columns.get(longindex.unwrap()) {
            if let Ok(value) = long_value.parse::<f32>() {
                long_values.push(value);
            }
        }
    }
}
