use regex::Regex;

const REPEAT_BATCH_PREXIT: &'static str = "REPEAT_BATCH:";

pub fn is_batch_repeat_request(message: &str) -> bool {
    message.starts_with(REPEAT_BATCH_PREXIT)
}

pub fn get_batch_id_for_repeat(message: &str) -> Option<u32> {
    let re = Regex::new(&("^".to_owned() + REPEAT_BATCH_PREXIT + r"(?P<id>\d+)$")).unwrap();
    re.captures(&message)
        .map(|c| c["id"].parse().unwrap())
}
