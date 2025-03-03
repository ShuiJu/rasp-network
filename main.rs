use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
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

fn handle_client(stream: TcpStream, count: usize, times: usize) {
    let (reader, writer) = (stream.try_clone().expect("Failed to clone stream"), stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut buffer = String::new();

    for i in 0..times {
        // 1. 生成随机数数组
        let numbers = generate_random_numbers(count, rand::random());
        let numbers_str = numbers
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join(",");

        // 2. 发送随机数数组给客户端
        writer
            .write_all(format!("{}\n", numbers_str).as_bytes())
            .expect("Failed to send numbers");
        writer.flush().expect("Failed to flush writer");
        println!("Sent random numbers to client for iteration {}", i + 1);

        // 3. 保存随机数数组到文件
        save_to_file(&format!("server_numbers_{}.txt", i), &numbers);

        // 4. 使用冒泡排序对随机数数组进行排序
        println!("Sorting using bubble sort...");
        let sorted = bubble_sort(numbers);
        save_to_file(&format!("server_sorted_{}.txt", i), &sorted);

        // 5. 等待客户端完成排序
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
}

fn start_server(port: u16) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind to port");
    println!("Starting server on 0.0.0.0:{}", port);

    let (mut stream, addr) = listener.accept().expect("Failed to accept client");
    println!("New client connected: {:?}", addr);

    print!("请输入随机数个数: ");
    io::stdout().flush().unwrap();
    let mut count_input = String::new();
    io::stdin().read_line(&mut count_input).unwrap();
    let count: usize = count_input.trim().parse().expect("Invalid number");

    print!("请输入排序次数: ");
    io::stdout().flush().unwrap();
    let mut times_input = String::new();
    io::stdin().read_line(&mut times_input).unwrap();
    let times: usize = times_input.trim().parse().expect("Invalid number");

    handle_client(stream, count, times);
}

fn start_client(server_ip: String, port: u16) {
    // Connect to the server
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

    // Split the stream into reader and writer
    let (reader, writer) = (stream.try_clone().expect("Failed to clone stream"), stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut buffer = String::new();

    for i in 0.. {
        // 1. 接收服务器发送的随机数数组
        match reader.read_line(&mut buffer) {
            Ok(_) => {
                let trimmed = buffer.trim();
                let numbers: Vec<u32> = trimmed
                    .split(',')
                    .map(|s| s.parse::<u32>().expect("Failed to parse number"))
                    .collect();

                // 2. 保存随机数数组到文件
                save_to_file(&format!("client_numbers_{}.txt", i), &numbers);

                // 3. 使用快速排序对随机数数组进行排序
                println!("Sorting locally using quick sort...");
                let sorted = quick_sort(numbers);
                save_to_file(&format!("client_sorted_{}.txt", i), &sorted);

                // 4. 通知服务器排序完成
                writer
                    .write_all(b"Client sort completed\n")
                    .expect("Failed to send message");
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

fn parse_input(input: &str) -> Result<(usize, usize, u64), String> {
    let parts: Vec<&str> = input.split(',').collect();
    if parts.len() == 3 {
        let count = parts[0].parse::<usize>().map_err(|_| "Invalid count".to_string())?;
        let times = parts[1].parse::<usize>().map_err(|_| "Invalid times".to_string())?;
        let seed = parts[2].parse::<u64>().map_err(|_| "Invalid seed".to_string())?;
        Ok((count, times, seed))
    } else {
        Err("Invalid input format".to_string())
    }
}

fn run_sorting_iterations(
    reader: &mut BufReader<TcpStream>,
    writer: &mut BufWriter<TcpStream>,
    count: usize,
    times: usize,
    initial_seed: u64,
) {
    let mut seed = initial_seed;

    for i in 0..times {
        println!(
            "Client iteration {}: Generating {} random numbers with seed {}",
            i + 1,
            count,
            seed
        );
        let numbers = generate_random_numbers(count, seed);
        save_to_file(&format!("client_numbers_{}.txt", i), &numbers);
        println!("Sorting locally using quick sort...");
        let sorted = quick_sort(numbers);
        save_to_file(&format!("client_sorted_{}.txt", i), &sorted);

        // 通知服务器排序完成
        writer
            .write_all(b"Client sort completed\n")
            .expect("Failed to send message");
        writer.flush().expect("Failed to flush writer");

        if i < times - 1 {
            // 生成新种子并发送给服务器
            let new_seed: u64 = rand::random();
            writer
                .write_all(format!("{}\n", new_seed).as_bytes())
                .expect("Failed to send new seed");
            writer.flush().expect("Failed to flush writer");
            println!("Sent new seed to server: {}", new_seed);

            // 等待服务器确认继续
            let mut response = String::new();
            match reader.read_line(&mut response) {
                Ok(_) => {
                    if response.trim() != "Continue" {
                        eprintln!("Unexpected response from server: {}", response.trim());
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read from server: {}", e);
                    break;
                }
            }

            seed = new_seed;
        }
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
