use std::env;
use std::time::{Duration, SystemTime};
use async_std::task;
use zenoh::net::*;
use zenoh::net::ResKey::*;

const N: u128 = 100000;

fn print_stats(start: SystemTime, stop: SystemTime) {
    let elapsed = stop.duration_since(start).unwrap();
    let thpt = (N as f64) / elapsed.as_secs_f64();
    println!("{} msgs/sec", thpt);
}


fn main() {
    task::block_on( async {
        let mut args: Vec<String> = env::args().collect();

        args.pop(); // ignore arg[0] (exe name)
        let locator = args.pop().unwrap_or("".to_string());

        let session = open(&locator, None).await.unwrap();

        let reskey = RId(session.declare_resource(&RName("/test/thr".to_string())).await.unwrap());

        let mut count = 0u128;
        let mut start = SystemTime::now();
        let mut stop  = SystemTime::now();

        let sub_info = SubInfo {
            reliability: Reliability::Reliable,
            mode: SubMode::Push,
            period: None
        };
        let sub = session.declare_subscriber(&reskey, &sub_info,
            move |_res_name: &str, _payload: &[u8], _data_info: &[u8]| {
                if count == 0 {
                    start = SystemTime::now();
                    count = count + 1;
                } else if count < N {
                    count = count + 1;
                } else {
                    stop = SystemTime::now();
                    print_stats(start, stop);
                    count = 0;
                }
            }
        ).await.unwrap();

        std::thread::sleep(Duration::from_secs(60));

        session.undeclare_subscriber(sub).await.unwrap();
        session.close().await.unwrap();
    })
}
