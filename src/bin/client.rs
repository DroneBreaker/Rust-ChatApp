use async_std::io::{self, BufRead};
use async_std::{prelude::*, task};
use async_std::net;
use std::sync::Arc;
use chat::utils::{self, ChatResult};
use chat::{Client, Server};
use futures_lite::future::FutureExt; // Assuming you have defined Client type in the chat module

fn get_value(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();

    if input.is_empty() {
        return None;
    }

    match input.find(char::is_whitespace) {
        Some(whitespace) => Some((&input[0..whitespace], &input[whitespace..])),
        None => Some((input, ""))
    }
}

fn parse_input(line: &str) -> Option<Client> {
    let (input, remainder) = get_value(line)?;

    if input == "join" {
        let (chat, remainder) = get_value(remainder)?;

        if !remainder.trim_start().is_empty() {
            return None;
        }

        return Some(Client::Join {
            chat_name: Arc::new(chat.to_string()),
        });
    } else if input == "post" {
        let (chat, remainder) = get_value(remainder)?;
        let message = remainder.trim_start().to_string();

        return Some(Client::Post {
            chat_name: Arc::new(chat.to_string()),
            message: Arc::new(message),
        });
    } else {
        println!("Unrecognized input: {:?}", line);
        return None;
    }
}

async fn send(mut send: net::TcpStream) -> ChatResult<()> {
    println!("Options: \n join CHAT \n post CHAT MESSAGE");

    let mut options = io::BufReader::new(io::stdin()).lines();

    while let Some(option_result) = options.next().await {
        let opt = match option_result {
            Ok(opt) => opt,
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                continue;
            }
        };
        let req = match parse_input(&opt) {
            Some(req) => req,
            None => continue,
        };

        if let Err(e) = utils::send_json(&mut send, &req).await {
            eprintln!("Error sending data: {}", e);
            continue;
        }
        if let Err(e) = send.flush().await {
            eprintln!("Error flushing data: {}", e);
            continue;
        }
    }

    Ok(())
}


async fn message(server: net::TcpStream) -> ChatResult<()> {
    let buf = io::BufReader::new(server);

    let mut stream = utils::receive(buf);

    while let Some(msg) = stream.next().await {
        match msg? {
            Server::Message { chat_name, message } => {
                println!("Chat Name: {}\n, Message: {}\n", chat_name, message);
            }

            Server::Error(message) => println!("Error received: {}", message)
        }
    }

    Ok(())
}

fn main() -> ChatResult<()>{
    let addr = std::env::args().nth(1).expect("Address:PORT");

    task::block_on(async {
        let socket = net::TcpStream::connect(addr).await?;
        socket.set_nodelay(true);
        let send = send(socket.clone());
        let replies = message(socket);

        replies.race(send).await?;

        Ok(())
    })
} 