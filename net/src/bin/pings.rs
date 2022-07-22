use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use bincode::{Encode, Decode};
use net::{client, server};

fn main() -> anyhow::Result<()> {
    let counter = Arc::new(AtomicUsize::new(0));

    {
        let counter = counter.clone();

        let _server_packet_producer = server::start_server::<Proto, Proto, _>("0.0.0.0:33000".parse()?, move |packet, producer| {
            match packet {
                Proto::Ping(v) => {
                    producer.send(Proto::Ping(v)).unwrap();

                    counter.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }
    thread::sleep(Duration::from_millis(50));
    let client_packet_producer = client::start_client::<Proto, Proto, _>("0.0.0.0:33000".parse()?, |packet, producer| {
        match packet {
            Proto::Ping(v) => {
                producer.send(Proto::Ping(v)).unwrap();
            }
        }
    });
    thread::sleep(Duration::from_millis(50));

    let start = Instant::now();

    for _ in 0..1000 {
        client_packet_producer.send(Proto::Ping([10; 100000])).unwrap();
    }


    thread::sleep(Duration::from_secs(5));

    let count = counter.load(Ordering::Relaxed);
    let time = start.elapsed();
    let mps = count as f64 / time.as_secs_f64();

    println!("{} messages", count);
    println!("{} seconds", time.as_secs_f64());
    println!("{} messages per second", mps);

    Ok(())
}

#[derive(Encode, Decode, Clone, Debug)]
enum Proto {
    Ping([u8; 100000])
}
