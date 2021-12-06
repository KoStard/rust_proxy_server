use regex::Regex;
use reqwest::StatusCode;

pub struct ProxyLogic {}

impl ProxyLogic {
    pub fn process_message(message: &str) -> Result<String, String> {
        let re = Regex::new(r"^GET:(?P<url>.+)$").unwrap();
        println!("The message is {}", message);
        let cap = re.captures(message);
        if let Some(capture) = cap {
            Ok(capture["url"].to_owned())
        } else {
            Err("Invalid message structure! Use GET:URL format.\n".to_owned())
        }
    }

    pub async fn generate_content_to_send(url: &str) -> Result<Vec<u8>, String> {
        println!("The url is {}", url);
        let result = reqwest::get(url)
            .await
            .map_err(|e| e.to_string())?;
        if result.status() == StatusCode::OK {
            Ok(result.bytes()
                .await
                .map_err(|e| e.to_string())?
                .to_vec())
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
            Ok(generated_response.into_bytes())
        }
    }
}