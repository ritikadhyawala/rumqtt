use rumqttc::{self, MqttOptions, Incoming, QoS, Client};
use std::time::{Instant, Duration};
use std::error::Error;
use std::thread;

mod common;


#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() {
    pretty_env_logger::init();
    let guard = pprof::ProfilerGuard::new(250).unwrap();
    start("rumqtt-sync", 100, 1_000_000).unwrap();
    common::profile("bench.pb", guard);
}

pub fn start(id: &str, payload_size: usize, count: usize) -> Result<() , Box<dyn Error>> {
    let mut mqttoptions = MqttOptions::new(id, "localhost", 1883);
    mqttoptions.set_keep_alive(20);
    mqttoptions.set_max_request_batch(10);

    // NOTE More the inflight size, better the perf
    mqttoptions.set_inflight(100);

    let (client, mut connection) = Client::new(mqttoptions, 10);
    thread::spawn(move || {
        let mut client = client;
        requests(count, payload_size, &mut client);
        thread::sleep(Duration::from_secs(1));
    });

    let mut acks_count = 0;
    let start = Instant::now();
    for o in connection.iter()  {
        let (notification, _) = o?;
        let notification = match notification {
            Some(n) => n,
            None => continue
        };

        match notification {
            Incoming::PubAck(_puback) => {
                acks_count += 1;
            }
            _notification => {
                continue;
            }
        };

        // println!("{}, {}", count, acks_count);
        if acks_count == count {
            break;
        }
    }

    let elapsed_ms = start.elapsed().as_millis();
    let throughput = (acks_count as usize * 1000) / elapsed_ms as usize;
    println!("Id = {}, Messages = {}, Payload (bytes) = {}, Throughput (messages/sec) = {}",
             id,
             acks_count,
             payload_size,
             throughput,
    );
    Ok(())
}

fn requests(count: usize, payload_size: usize, client: &mut Client) {
    for i in 0..count {
        let mut payload = vec![1; payload_size];
        payload[0] = (i % 255) as u8;
        if let Err(e) = client.publish("hello/world", QoS::AtLeastOnce, false, payload) {
            println!("Client error: {:?}", e);
            break;
        }
    }
}
