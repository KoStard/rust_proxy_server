use regex::Regex;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;

    // accept connections and process them serially
    for stream in listener.incoming() {
        let mut unwrapped_stream = stream?;
        let res = handle_client(&mut unwrapped_stream).await;
        if let Err(e) = res {
            unwrapped_stream.write(format!("Error occured: {}\n", e).as_bytes())?;
        }
    }
    Ok(())
}



async fn handle_client(stream: &mut TcpStream) -> Result<(), String> {
    println!("{:?}", stream);
    println!("Connection established");
    const MAX_BATCH_SIZE: usize = 500;
    const MAX_MESSAGE_SIZE: usize = 10000;
    let mut overall_message = String::new();
    loop {
        let mut buffer = [0; MAX_BATCH_SIZE];
        let count = stream.read(&mut buffer).map_err(|e| e.to_string())?;
        let message = String::from_utf8_lossy(&buffer);
        let non_trimmed_length = message.len();
        let filler_trimmed_message = message.trim_end_matches(|c: char| c.eq(&'\u{0}'));
        let filler_trimmed_message_length = filler_trimmed_message.len();

        let fully_trimmed_message = filler_trimmed_message.trim_end();
        overall_message.push_str(fully_trimmed_message);

        if overall_message.len() > MAX_MESSAGE_SIZE {
            return Err(format!(
                "The maximum length of input message was exceeded. The limit is {}",
                MAX_MESSAGE_SIZE
            ));
        }

        if count == 0
            || (non_trimmed_length != filler_trimmed_message_length
                && filler_trimmed_message.ends_with('\n'))
        {
            println!("Finished the message {}.", overall_message);
            break;
        }
    }

    let re = Regex::new(r"^GET:(?P<url>.+)$").unwrap();
    let cap = re.captures(&overall_message);
    if cap.is_none() {
        return Err("Invalid message structure! Use GET:URL format.".to_owned());
    }
    let unwrapped_capture = cap.unwrap();
    let url = unwrapped_capture["url"].to_owned();

    // stream
    //     .write(format!("Got url {}\n", &url).as_bytes())
    //     .map_err(|e| e.to_string())?;

    // TODO url validation

    let result = reqwest::get(&url).await.map_err(|e| e.to_string())?;

    if result.status() == 200 {
        stream
            .write(&result.bytes().await.map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    } else {
        let generated_response = format!(
            r#"
                <html lang="en">
                <head>
                    <meta charset="UTF-8">
                    <meta http-equiv="X-UA-Compatible" content="IE=edge">
                    <meta name="viewport" content="width=device-width, initial-scale=1.0">
                    <title>Error {}</title>
                </head>
                <body>
                    <div style="position: absolute;top: 50%;left: 50%;transform: translate(-50%, -50%);">Received {} error from {} url</div>
                </body>
                </html>
        "#,
            result.status(),
            result.status(),
            url
        );
        stream
            .write(generated_response.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    // let status = result.status();
    // if status != 200 {
    //     return Err(result.bytes())
    // }
    // let result_text = result
    //     .text()
    //     .await
    //     .map_err(|e| e.to_string())?;

    Ok(())
}
