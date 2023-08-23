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
    let mut MAFV: Option<usize> = None;
    let mut MAFI: Option<usize> = None;
    let mut STFTI: Option<usize> = None;
    let mut LTFTI: Option<usize> = None;

    for (i, e) in column_headers.into_iter().enumerate() {
        match e.as_str() {
            "MAF Voltage (V)" => {
                MAFV = Some(i);
            }
            "Mass Airflow (g/s)" => {
                MAFI = Some(i);
            }
            "Short Term FT (%)" => {
                STFTI = Some(i);
            }
            "Long Term FT (%)" => {
                LTFTI = Some(i);
            }
            _ => {}
        }
    }

    if MAFV.is_some() && MAFI.is_some() && STFTI.is_some() && LTFTI.is_some() {
        println!("All matches were successful!");
        println!("MAF V index: {:?}", MAFV.unwrap());
        println!("Mass A index: {:?}", MAFI.unwrap());
        println!("Short T index: {:?}", STFTI.unwrap());
        println!("Long T index: {:?}", LTFTI.unwrap());
    } else {
        println!("The following matches were not successful:");
        if MAFV.is_none() {
            println!("- MAF V");
        }
        if MAFI.is_none() {
            println!("- Mass A");
        }
        if STFTI.is_none() {
            println!("- Short T");
        }
        if LTFTI.is_none() {
            println!("- Long T");
        }
    }
}
