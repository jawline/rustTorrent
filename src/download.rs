use torrent::{from_file, prepare};
use tracker::{TrackerData, connect};

pub fn download(filename: &str) {
    println!("Loading {}", filename);
    let root = from_file(&filename).unwrap();
    let info = prepare(&root).unwrap();

    println!("Loading {}", info.name);

    let (ctrl, data) = connect(&info);

    loop {
        let tracker_data = data.try_recv();
        if let Ok(TrackerData::Close) = tracker_data {
            println!("Tracker Closed");
            break;
        }
    } 
}
