use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::process::Command;
use std::time::{Duration, Instant};
use std::fs::File;
use clap::Parser;
use rand;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    server: bool,
    #[arg(long)]
    client: bool,
    #[arg(long)]
    server_ip: Option<String>,
    #[arg(long, default_value = "8080")]
    port: u16,
}

fn generate_random_numbers(count: usize, _seed: u64) -> Vec<u32> {
    (0..count).map(|_| rand::random()).collect()
}

fn bubble_sort(mut arr: Vec<u32>) -> Vec<u32> {
    let start = Instant::now();
    let len = arr.len();
    for i in 0..len {
        for j in 0..len - i - 1 {
            if arr[j] > arr[j + 1] {
                arr.swap(j, j + 1);
            }
        }
    }
    let duration = start.elapsed();
    println!("Bubble sort completed in: {:?}", duration);
    arr
}

fn quick_sort(mut arr: Vec<u32>) -> Vec<u32> {
    let start = Instant::now();
    arr.sort_unstable();
    let duration = start.elapsed();
    println!("Quick sort completed in: {:?}", duration);
    arr
}

fn save_to_file(filename: &str, data: &[u32]) {
    let mut file = File::create(filename).expect("Failed to create file");
    for num in data {
        writeln!(file, "{}", num).expect("Failed to write to file");
    }
}

fn read_temperature() -> Option<f32> {
    let output = Command::new("python3")
        .arg("read_temp.py")
        .output()
        .expect("Failed to execute temperature script");

    if output.status.success() {
        let temp_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(temp) = temp_str.trim().parse::<f32>() {
            return Some(temp);
        }
    }
    None
}

fn handle_client(stream: TcpStream, count: usize, times: usize) {
    let (reader, writer) = (stream.try_clone().expect("Failed to clone stream"), stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut buffer = String::new();

    for i in 0..times {
        let numbers = generate_random_numbers(count, rand::random());
        let numbers_str = numbers
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join(",");

        writer
            .write_all(format!("{}\n", numbers_str).as_bytes())
            .expect("Failed to send numbers");
        writer.flush().expect("Failed to flush writer");
        // println!("Sent random numbers to client for iteration {}", i + 1);

        // 读取温度
        if let Some(temp) = read_temperature() {
            println!("Server temperature: {:.2}°C", temp);
        } else {
            println!("Failed to read server temperature.");
        }

        save_to_file(&format!("server_numbers_{}.txt", i), &numbers);

        // println!("Sorting using bubble sort...");
        let sorted = bubble_sort(numbers);
        save_to_file(&format!("server_sorted_{}.txt", i), &sorted);

        match reader.read_line(&mut buffer) {
            Ok(_) => {
                let trimmed = buffer.trim();
                if trimmed == "Client sort completed" {
                    println!("Client completed iteration {}", i + 1);
                } else {
                    eprintln!("Unexpected message from client: {}", trimmed);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Failed to read from client: {}", e);
                break;
            }
        }

        buffer.clear();
    }

    println!("All iterations completed. Closing connection.");
    writer.write_all(b"END\n").expect("Failed to send END message");
    writer.flush().expect("Failed to flush writer");
}

fn start_server(port: u16) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind to port");
    println!("Starting server on 0.0.0.0:{}", port);

    let (mut stream, addr) = listener.accept().expect("Failed to accept client");
    println!("New client connected: {:?}", addr);

    print!("Please enter number of random numbers per loop: ");
    io::stdout().flush().unwrap();
    let mut count_input = String::new();
    io::stdin().read_line(&mut count_input).unwrap();
    let count: usize = count_input.trim().parse().expect("Invalid number");

    print!("Please enter number of loops for sorting: ");
    io::stdout().flush().unwrap();
    let mut times_input = String::new();
    io::stdin().read_line(&mut times_input).unwrap();
    let times: usize = times_input.trim().parse().expect("Invalid number");

    handle_client(stream, count, times);
}

fn start_client(server_ip: String, port: u16) {
    let stream = match TcpStream::connect(format!("{}:{}", server_ip, port)) {
        Ok(stream) => {
            println!("Connecting to server at {}:{}", server_ip, port);
            stream
        }
        Err(e) => {
            eprintln!("Could not connect to server: {}", e);
            return;
        }
    };

    let (reader, writer) = (stream.try_clone().expect("Failed to clone stream"), stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut buffer = String::new();

    for i in 0.. {
        match reader.read_line(&mut buffer) {
            Ok(0) => {
                // 服务器关闭连接 (EOF)
                println!("Server closed the connection.");
                break;
            }
            Ok(_) => {
                let trimmed = buffer.trim();
                if trimmed == "END" {
                    println!("Received END signal from server. Exiting...");
                    break;
                }
        
                let numbers: Vec<u32> = trimmed
                    .split(',')
                    .filter_map(|s| s.parse::<u32>().ok()) // 过滤掉解析失败的值
                    .collect();
                
                if numbers.is_empty() {
                    eprintln!("Received an empty or invalid number sequence. Exiting...");
                    break;
                }
        
                save_to_file(&format!("client_numbers_{}.txt", i), &numbers);
        
                println!("Sorting locally using quick sort...");
                let sorted = quick_sort(numbers);
                save_to_file(&format!("client_sorted_{}.txt", i), &sorted);
        
                writer.write_all(b"Client sort completed\n").expect("Failed to send message");
                writer.flush().expect("Failed to flush writer");
            }
            Err(e) => {
                eprintln!("Failed to read from server: {}", e);
                break;
            }
        }
        

        buffer.clear();
    }

    println!("All iterations completed. Closing connection.");
}

fn main() {
    let args = Args::parse();
    if args.server {
        start_server(args.port);
    } else if args.client {
        if let Some(server_ip) = args.server_ip {
            start_client(server_ip, args.port);
        } else {
            eprintln!("Error: --client requires --server-ip");
        }
    } else {
        eprintln!("Error: Must specify either --server or --client");
    }
}