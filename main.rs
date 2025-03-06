use std::io::{self, BufReader, BufWriter, Write, Read, BufRead};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};
use std::fs::{File, OpenOptions};
use std::path::Path;
use clap::Parser;
use rand;
use bincode;
use csv::Writer;


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
    #[arg(long)] // 新增 --no-log 选项
    no_log: bool,
}

fn generate_random_numbers(count: usize) -> Vec<u32> {
    // println!("Executing: generate_random_numbers");
    println!("Generating {} random numbers...", count);
    let numbers: Vec<u32> = (0..count).map(|_| rand::random()).collect();
    // println!("Generated random numbers: {:?}", &numbers[..10]);
    numbers
}

fn bubble_sort(mut arr: Vec<u32>) -> Vec<u32> {
    // println!("Executing: bubble_sort");
    println!("Starting bubble sort...");
    let start = Instant::now();
    let len = arr.len();
    for i in 0..len {
        for j in 0..len - i - 1 {
            if arr[j] > arr[j + 1] {
                // println!("Swapping elements at indices {} and {}", j, j + 1);
                arr.swap(j, j + 1);
            }
        }
    }
    let duration = start.elapsed();
    println!("Bubble sort completed in: {:?}", duration);
    arr
}

fn quick_sort(mut arr: Vec<u32>) -> Vec<u32> {
    // println!("Executing: quick_sort");
    println!("Starting quick sort...");
    let start = Instant::now();
    arr.sort_unstable();
    let duration = start.elapsed();
    println!("Quick sort completed in: {:?}", duration);
    arr
}

fn save_to_file(filename: &str, data: &[u32]) -> Result<(), Box<dyn std::error::Error>> {
    // println!("Executing: save_to_file");
    // println!("Saving data to file: {}", filename);
    let mut file = File::create(filename)?;
    for num in data {
        writeln!(file, "{}", num)?;
    }
    // println!("Data saved to file: {}", filename);
    Ok(())
}

fn read_temperature() -> Option<f32> {
    // println!("Executing: read_temperature");
    // println!("Reading temperature...");
    let output = Command::new("python3")
        .arg("read_temp.py")
        .output()
        .expect("Failed to execute temperature script");
    if output.status.success() {
        let temp_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(temp) = temp_str.trim().parse::<f32>() {
            println!("Temperature read: {:.2}°C", temp);
            return Some(temp);
        }
    }
    println!("Failed to read temperature.");
    None
}

fn log_temperature(csv_writer: &mut Writer<File>, iteration: usize, temperature: f32) -> Result<(), Box<dyn std::error::Error>> {
    // println!("Executing: log_temperature");
    // println!("Logging temperature for iteration {}: {:.2}°C", iteration, temperature);
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    csv_writer.write_record(&[
        timestamp.to_string(),
        iteration.to_string(),
        temperature.to_string(),
    ])?;
    csv_writer.flush()?;
    // println!("Temperature logged for iteration {}", iteration);
    Ok(())
}
// 在send_data函数中添加详细日志
fn send_data(writer: &mut BufWriter<TcpStream>, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // println!("[NET][OUT] Sending DATA packet | Flag: 0x01 | Length: {}", data.len());
    writer.write_all(&[0x01])?;
    writer.write_all(&(data.len() as u32).to_le_bytes())?;
    // println!("[NET][OUT] Payload (hex): {}", bytes_to_hex(&data[..data.len().min(16)]));
    writer.write_all(data)?;
    writer.flush()?;
    Ok(())
}



fn send_end_signal(writer: &mut BufWriter<TcpStream>) -> Result<(), Box<dyn std::error::Error>> {
    // println!("Executing: send_end_signal");
    println!("Sending END signal...");
    // 发送标志位（0x02 表示控制信号）
    println!("Sending flag: 0x02");
    writer.write_all(&[0x02])?;

    // 发送数据长度（0 表示没有数据内容）
    println!("Sending data length: 0");
    writer.write_all(&0u32.to_le_bytes())?;
    writer.flush()?;

    println!("END signal sent.");
    Ok(())
}

fn receive_data(reader: &mut BufReader<TcpStream>) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    // 清理缓冲区改为兼容方式
    let available = reader.buffer().len();
    if available > 0 {
        reader.consume(available);
    }
    
    // // 清空缓冲区残留数据
    // reader.consume(reader.buffer().len());
    
    let mut flag = [0u8; 1];
    reader.read_exact(&mut flag)?;

    match flag[0] {
        0x01 => {
            // 处理数据包
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            let data_len = u32::from_le_bytes(len_buf);
            
            // 增加长度校验
            if data_len > 1_000_000 {
                return Err("Suspicious data length".into());
            }
            
            let mut data = vec![0u8; data_len as usize];
            reader.read_exact(&mut data)?;
            Ok(Some(data))
        }
        0x02 => {
            // println!("[PROTOCOL] Received END signal");
            Ok(None)
        }
        0x03 => {
            // println!("[PROTOCOL] Server ready");
            Ok(None)
        }
        0x04 => {
            // println!("[PROTOCOL] Client ready");
            Ok(None)
        }
        _ => {
            println!("[PROTOCOL] Invalid flag: 0x{:02X}", flag[0]);
            Err("Protocol violation".into())
        }
    }
}


fn handle_client(stream: TcpStream, count: usize, times: usize, no_log: bool) -> Result<(), Box<dyn std::error::Error>> {
    // println!("[SERVER] Handling client connection");
    let (reader, writer) = (stream.try_clone()?, stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    // 发送配置
    let config = (count, times);
    let config_bytes = bincode::serialize(&config)?;
    send_data(&mut writer, &config_bytes)?;

    // 初始化CSV日志
    let mut csv_writer = if !no_log {
        let csv_path = "server_temps.csv";
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(csv_path)?;
        let mut writer = csv::Writer::from_writer(file);
        if Path::new(csv_path).metadata()?.len() == 0 {
            writer.write_record(&["timestamp", "iteration", "temperature"])?;
        }
        Some(writer)
    } else {
        None
    };

    let connection_active = true;
    let mut current_iter = 0;

    while connection_active && current_iter < times {
        // println!("[SERVER] Starting iteration {}", current_iter);
        if current_iter % 2 == 0 {
            // Server's turn to send data
            let numbers = generate_random_numbers(count);
            // println!("[SERVER] Generated {} numbers", numbers.len());
            
            if let Err(e) = send_data(&mut writer, &bincode::serialize(&numbers)?) {
                println!("[SERVER] Send error: {}", e);
                break;
            }
            
            save_to_file(&format!("server_numbers_{}.txt", current_iter), &numbers)?;
            // Temperature logging
            if let Some(temp) = read_temperature() {
                if let Some(writer) = &mut csv_writer {
                    log_temperature(writer, current_iter, temp)?;
                }
            }
            let sorted = bubble_sort(numbers);
            save_to_file(&format!("server_sorted_{}.txt", current_iter), &sorted)?;
        } else {
            // Client's turn to send data
            // println!("[SERVER] Waiting for client data...");
            let start = Instant::now();
            let timeout = Duration::from_secs(10);
            let mut received = false;
            
            while start.elapsed() < timeout && !received {
                // Send SERVER_READY every 2 seconds
                if start.elapsed().as_secs() % 2 == 0 {
                    // println!("[SERVER] Sending SERVER_READY (0x03)");
                    writer.write_all(&[0x03])?;
                    writer.flush()?;
                }
                
                match receive_data(&mut reader) {
                    Ok(Some(data)) => {
                        let numbers: Vec<u32> = bincode::deserialize(&data)?;
                        // println!("[SERVER] Received {} numbers", numbers.len());
                        // Temperature logging
                        if let Some(temp) = read_temperature() {
                            if let Some(writer) = &mut csv_writer {
                                log_temperature(writer, current_iter, temp)?;
                            }
                        }
                        let sorted = bubble_sort(numbers);
                        save_to_file(&format!("server_sorted_{}.txt", current_iter), &sorted)?;
                        received = true;
                    }
                    Ok(None) => {
                        // Control signal handled
                        continue;
                    }
                    Err(e) => {
                        println!("[SERVER] Receive error: {}", e);
                        break;
                    }
                }
                
                std::thread::sleep(Duration::from_millis(100));
            }
            
            if !received {
                println!("[SERVER] Timeout waiting for client data");
                break;
            }
        }

        

        // Wait for confirmation
        // println!("[SERVER] Waiting for client confirmation");
        let mut confirm = [0u8; 1];
        if reader.read_exact(&mut confirm).is_ok() {
            // println!("[SERVER] Received confirmation: 0x{:02X}", confirm[0]);
        }

        current_iter += 1;
    }

    // Send END signal
    send_end_signal(&mut writer)?;
    println!("[SERVER] Connection closed gracefully");
    Ok(())
}


fn start_client(server_ip: String, port: u16, no_log: bool) -> Result<(), Box<dyn std::error::Error>> {
    // println!("[CLIENT] Connecting to {}:{}", server_ip, port);
    let stream = TcpStream::connect(format!("{}:{}", server_ip, port))?;
    let (reader, writer) = (stream.try_clone()?, stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    // 初始化CSV日志
    let mut csv_writer = if !no_log {
        let csv_path = "client_temps.csv";
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(csv_path)?;
        let mut writer = csv::Writer::from_writer(file);
        if Path::new(csv_path).metadata()?.len() == 0 {
            writer.write_record(&["timestamp", "iteration", "temperature"])?;
        }
        Some(writer)
    } else {
        None
    };

    // 接收配置
    let (count, times) = match receive_data(&mut reader)? {
        Some(data) => bincode::deserialize(&data)?,
        None => return Err("Failed to receive config".into()),
    };

    for current_iter in 0..times {
        // println!("[CLIENT] Starting iteration {}", current_iter);

        if current_iter % 2 == 1 {
            // Client's turn to send data
            // println!("[CLIENT] Sending CLIENT_READY (0x04)");
            writer.write_all(&[0x04])?;
            writer.flush()?;

            // Wait for SERVER_READY
            let mut flag = [0u8; 1];
            match reader.read_exact(&mut flag) {
                Ok(_) => {
                    if flag[0] != 0x03 {
                        // println!("[CLIENT] Expected 0x03, got 0x{:02X}", flag[0]);
                        continue;
                    }
                    // println!("[CLIENT] Received SERVER_READY");
                    
                    let numbers = generate_random_numbers(count);
                    if let Err(e) = send_data(&mut writer, &bincode::serialize(&numbers)?) {
                        println!("[CLIENT] Send error: {}", e);
                        break;
                    }
                    save_to_file(&format!("client_numbers_{}.txt", current_iter), &numbers)?;
                    // Temperature logging
                    if let Some(temp) = read_temperature() {
                        if let Some(writer) = &mut csv_writer {
                            log_temperature(writer, current_iter, temp)?;
                        }
                    }
                    let sorted = quick_sort(numbers);
                    save_to_file(&format!("client_sorted_{}.txt", current_iter), &sorted)?;
                }
                Err(e) => {
                    println!("[CLIENT] Read error: {}", e);
                    break;
                }
            }
        } else {
            // Server's turn to send data
            // println!("[CLIENT] Waiting for server data...");
            let start = Instant::now();
            let timeout = Duration::from_secs(10);
            let mut received = false;
            
            while start.elapsed() < timeout && !received {
                match receive_data(&mut reader) {
                    Ok(Some(data)) => {
                        let numbers: Vec<u32> = bincode::deserialize(&data)?;
                        // println!("[CLIENT] Received {} numbers", numbers.len());
                        // Temperature logging
                        if let Some(temp) = read_temperature() {
                            if let Some(writer) = &mut csv_writer {
                                log_temperature(writer, current_iter, temp)?;
                            }
                        }
                        let sorted = quick_sort(numbers);
                        save_to_file(&format!("client_sorted_{}.txt", current_iter), &sorted)?;
                        received = true;
                    }
                    Ok(None) => {
                        // Control signal handled
                        continue;
                    }
                    Err(e) => {
                        println!("[CLIENT] Receive error: {}", e);
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            
            if !received {
                println!("[CLIENT] Timeout waiting for server data");
                break;
            }
        }

        

        // Send confirmation
        // println!("[CLIENT] Sending confirmation (0x01)");
        writer.write_all(&[0x01])?;
        writer.flush()?;
    }

    // Send END signal
    send_end_signal(&mut writer)?;
    println!("[CLIENT] Connection closed gracefully");
    Ok(())
}


fn start_server(port: u16, no_log: bool) {
    // println!("Executing: start_server");
    println!("Starting server...");
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).expect("Failed to bind to port");
    println!("Starting server on 0.0.0.0:{}", port);
    let (stream, addr) = listener.accept().expect("Failed to accept client");
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

    if let Err(e) = handle_client(stream, count, times, no_log) {
        eprintln!("Error handling client: {}", e);
    }
}

// // 添加十六进制转换工具函数
// fn bytes_to_hex(bytes: &[u8]) -> String {
//     bytes.iter()
//         .map(|b| format!("{:02X}", b))
//         .collect::<Vec<String>>()
//         .join(" ")
// }


fn main() {
    // println!("Executing: main");
    // println!("Starting program...");
    let args = Args::parse();
    if args.server {
        start_server(args.port, args.no_log);
    } else if args.client {
        if let Some(server_ip) = args.server_ip {
            if let Err(e) = start_client(server_ip, args.port, args.no_log) {
                eprintln!("Client error: {}", e);
            }
        } else {
            eprintln!("Error: --client requires --server-ip");
        }
    } else {
        eprintln!("Error: Must specify either --server or --client");
    }
}
