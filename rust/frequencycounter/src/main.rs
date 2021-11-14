use core::time;
use separator::Separatable;
use serialport::{available_ports, SerialPort};
use std::str;
use std::sync::Arc;
use std::time::SystemTime;
use std::{
    env, process,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

fn read_byte(port: &mut dyn SerialPort) -> u8 {
    let mut byte = [0u8];
    port.read(&mut byte).unwrap();
    byte[0]
}

fn is_digit(byte: u8) -> bool {
    byte >= b'0' && byte <= b'9'
}

/// read a number from the serial port
/// numbers are separated by newline (any combination and count of \r and \n)
/// return 0 on illegal characters (line noise etc.)
fn read_number(port: &mut dyn SerialPort) -> u16 {
    let mut line: Vec<u8> = Vec::new();

    // wait until start of next number
    loop {
        let byte = read_byte(port);
        if is_digit(byte) {
            line.push(byte);
            break;
        }
    }

    // read all digits until newline
    loop {
        let byte = read_byte(port);
        if is_digit(byte) {
            line.push(byte);
        } else {
            break;
        }
    }

    // return converted to int
    if line.len() == 0 {
        0
    } else {
        match str::from_utf8(&line) {
            Ok(v) => match v.parse::<u16>() {
                Ok(v) => v,
                Err(_) => 0,
            },
            Err(_) => 0,
        }
    }
}

fn round_3digits(input: f64) -> f64 {
    (input * 1000.0).round() / 1000.0
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} port", args[0]);
        eprintln!("available ports:");
        let ports = available_ports().expect("No serial ports found!");
        for p in ports {
            eprintln!("{}", p.port_name);
        }
        process::exit(1);
    }

    // global counter
    let counter_thread = Arc::new(AtomicU64::new(0));
    let counter = counter_thread.clone();

    // open serial port
    let mut port = serialport::new(&args[1], 115_200)
        .timeout(Duration::from_millis(10000))
        .open()
        .expect("Failed to open port");

    // empty input buffer
    let to_read = port.bytes_to_read().unwrap();
    for _ in 0..to_read {
        read_byte(&mut *port);
    }

    // initial read to find first newline
    read_number(&mut *port);

    // start thread to accumulate the counter deltas
    thread::spawn(move || {
        let mut last16: u16 = 0;
        loop {
            let counter16 = read_number(&mut *port);
            let mut delta: i32 = (counter16 as i32) - (last16 as i32);
            if delta < 0 {
                // wraparound
                delta = (0x10000 - (last16 as i32)) + (counter16 as i32)
            };
            counter_thread.fetch_add(delta as u64, Ordering::SeqCst);
            last16 = counter16;
        }
    });

    // wait a bit until the thread has started and has read some numbers
    thread::sleep(time::Duration::from_millis(2000));

    // print frequency every second
    let start: u64 = counter.load(Ordering::SeqCst);
    let start_time = SystemTime::now();
    let mut last_frequency: f64 = 0.0;
    loop {
        let current: u64 = counter.load(Ordering::SeqCst);
        let elapsed = start_time.elapsed().unwrap().as_secs_f64();
        let counts = current - start;
        let frequency = round_3digits(counts as f64 / elapsed);
        let delta_frequency = round_3digits(last_frequency - frequency);
        println!(
            "time: {:.1} s, frequency: {} Hz, delta: {} Hz, counts: {}",
            elapsed,
            frequency.separated_string(),
            delta_frequency,
            counts.separated_string(),
        );
        last_frequency = frequency;
        thread::sleep(time::Duration::from_millis(1000));
    }
}
