use crossbeam_channel::{unbounded, Receiver};
use rouille::Request;
use rouille::Response;
use serde_json::json;
use std::io::prelude::*;
use std::io::BufReader;
use std::os::unix::net::{UnixListener, UnixStream};
use std::{env, fs::File, thread};

fn main() {
	let http_port = 1225;
	let socket_path = String::from(format!("/tmp/simple_socket_for_port_{}.sock", http_port));

	let args = env::args().skip(1).collect::<Vec<String>>();

	if args.len() > 0 {
		match args[0].as_str() {
			"listen" => {
				clean(&socket_path);
				start_server(socket_path, http_port);
			}
			"clean" => {
				clean(&socket_path);
			}

			"open" => match send_to_server(socket_path, &args[1..]) {
				Ok(_) => {}
				Err(e) => {
					dbg!(e);
				}
			},

			_ => {
				usage();
			}
		}
		return;
	} else {
		usage();
	}
}

fn usage() {
	println!("./me clean - (removes socket if it already exists)");
	println!("./me listen");
	println!("./me open file1 file2 file3...");
}

fn send_to_server(socket_path: String, files: &[String]) -> std::io::Result<()> {
	let mut stream = UnixStream::connect(socket_path)?;
	for (_, elem) in files.iter().enumerate() {
		stream.write_all(format!("{}\n", elem).as_bytes())?;
	}
	Ok(())
}

fn clean(socket_path: &String) {
	// clean up old socket just in case
	match std::fs::remove_file(&socket_path) {
		Ok(_) => {}
		Err(e) => {
			dbg!(e);
		}
	}
}

fn start_server(socket_path: String, http_port: i32) {
	let (s, r) = unbounded();

	thread::spawn(move || -> std::io::Result<()> {
		let listener = UnixListener::bind(socket_path)?;
		for stream in listener.incoming() {
			match stream {
				Ok(stream) => {
					let stream = BufReader::new(stream);
					for line_raw in stream.lines() {
						let line = line_raw.unwrap();
						s.send(line).unwrap();
					}
				}
				Err(err) => {
					println!("Error: {}", err);
					break;
				}
			}
		}
		Ok(())
	});

	println!("Starting server...");
	rouille::start_server(format!("localhost:{}", http_port), move |request| {
		if request.url().ends_with("ping") {
			return do_ping(&r);
		} else {
			match serve_file(format!(".{}", &request.url())) {
				Ok(response) => {
					return response;
				}
				Err(e) => {
					dbg!(e);
					return Response::empty_400();
				}
			}
		}
	});
}

fn do_ping(r: &Receiver<String>) -> Response {
	let files_to_open: Vec<_> = r.try_iter().collect();
	dbg!("Received: {}", &files_to_open);
	return Response::text(json!(&files_to_open).to_string());
}

fn serve_file(filepath: String) -> std::io::Result<Response> {
	println!("Trying to serve {}...", filepath);
	let mut file = File::open(filepath)?;
	let mut contents = String::new();
	file.read_to_string(&mut contents)?;
	return Ok(Response::text(contents));
}
