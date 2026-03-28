//! Corvus Nucleo-F401RE firmware — JSON-over-serial peripheral.
//!
//! Listens for newline-delimited JSON on USART2 (PA2=TX, PA3=RX).
//! USART2 is connected to ST-Link VCP — host sees /dev/ttyACM0 (Linux) or /dev/cu.usbmodem* (macOS).
//!
//! Protocol: same as Arduino/ESP32 — see docs/en/guides/hardware-peripherals-design.md

#![no_std]
#![no_main]

use core::fmt::Write;
use core::str;
use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::usart::{Config, Uart};
use heapless::String;
use {defmt_rtt as _, panic_probe as _};

/// Arduino-style pin 13 = PA5 (User LED LD2 on Nucleo-F401RE)
const LED_PIN: u8 = 13;

/// Build the `"key":` needle into a fixed buffer and return its length.
fn build_key_needle(key: &[u8], buf: &mut [u8; 32]) -> usize {
    buf[0] = b'"';
    let mut len = 1;
    for (i, &k) in key.iter().enumerate() {
        if i >= 30 {
            break;
        }
        buf[len] = k;
        len += 1;
    }
    buf[len] = b'"';
    buf[len + 1] = b':';
    len + 2
}

/// Parse a (possibly negative) integer from the start of `rest`.
fn parse_int_prefix(rest: &[u8]) -> i32 {
    let mut num: i32 = 0;
    let mut neg = false;
    let mut j = 0;
    if j < rest.len() && rest[j] == b'-' {
        neg = true;
        j += 1;
    }
    while j < rest.len() && rest[j].is_ascii_digit() {
        num = num * 10 + (rest[j] - b'0') as i32;
        j += 1;
    }
    if neg {
        -num
    } else {
        num
    }
}

/// Parse integer from JSON: "pin":13 or "value":1
fn parse_arg(line: &[u8], key: &[u8]) -> Option<i32> {
    let mut needle = [0u8; 32];
    let len = build_key_needle(key, &mut needle);
    let needle = &needle[..len];

    let line_len = line.len();
    if line_len < len {
        return None;
    }
    for i in 0..=line_len - len {
        if line[i..].starts_with(needle) {
            return Some(parse_int_prefix(&line[i + len..]));
        }
    }
    None
}

fn handle_ping(id_str: &str) -> String<128> {
    let mut resp: String<128> = String::new();
    let _ = write!(
        resp,
        "{{\"id\":\"{}\",\"ok\":true,\"result\":\"pong\"}}",
        id_str
    );
    resp
}

fn handle_capabilities(id_str: &str) -> String<128> {
    let mut resp: String<128> = String::new();
    let _ = write!(
        resp,
        "{{\"id\":\"{}\",\"ok\":true,\"result\":\"{{\\\"gpio\\\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13],\\\"led_pin\\\":13}}\"}}",
        id_str
    );
    resp
}

fn handle_gpio_read(id_str: &str, pin: i32) -> String<128> {
    let mut resp: String<128> = String::new();
    if pin == LED_PIN as i32 || (0..=13).contains(&pin) {
        let _ = write!(
            resp,
            "{{\"id\":\"{}\",\"ok\":true,\"result\":\"0\"}}",
            id_str
        );
    } else {
        let _ = write!(
            resp,
            "{{\"id\":\"{}\",\"ok\":false,\"result\":\"\",\"error\":\"Invalid pin {}\"}}",
            id_str, pin
        );
    }
    resp
}

fn handle_gpio_write(id_str: &str, pin: i32, value: i32) -> (String<128>, Option<(i32, i32)>) {
    let mut resp: String<128> = String::new();
    if value != 0 && value != 1 {
        let _ = write!(
            resp,
            "{{\"id\":\"{}\",\"ok\":false,\"result\":\"\",\"error\":\"Invalid value {}; expected 0 or 1\"}}",
            id_str, value
        );
        return (resp, None);
    }
    let led_action = if pin == LED_PIN as i32 {
        let _ = write!(
            resp,
            "{{\"id\":\"{}\",\"ok\":true,\"result\":\"done\"}}",
            id_str
        );
        Some((LED_PIN as i32, value))
    } else if (0..=13).contains(&pin) {
        let _ = write!(
            resp,
            "{{\"id\":\"{}\",\"ok\":true,\"result\":\"done\"}}",
            id_str
        );
        None
    } else {
        let _ = write!(
            resp,
            "{{\"id\":\"{}\",\"ok\":false,\"result\":\"\",\"error\":\"Invalid pin {}\"}}",
            id_str, pin
        );
        None
    };
    (resp, led_action)
}

fn handle_unknown_cmd(id_str: &str) -> String<128> {
    let mut resp: String<128> = String::new();
    let _ = write!(
        resp,
        "{{\"id\":\"{}\",\"ok\":false,\"result\":\"\",\"error\":\"Unknown command\"}}",
        id_str
    );
    resp
}

fn process_command(line_buf: &[u8], id_str: &str) -> (String<128>, Option<(i32, i32)>) {
    if has_cmd(line_buf, b"ping") {
        (handle_ping(id_str), None)
    } else if has_cmd(line_buf, b"capabilities") {
        (handle_capabilities(id_str), None)
    } else if has_cmd(line_buf, b"gpio_read") {
        let pin = parse_arg(line_buf, b"pin").unwrap_or(-1);
        (handle_gpio_read(id_str, pin), None)
    } else if has_cmd(line_buf, b"gpio_write") {
        let pin = parse_arg(line_buf, b"pin").unwrap_or(-1);
        let value = parse_arg(line_buf, b"value").unwrap_or(0);
        handle_gpio_write(id_str, pin, value)
    } else {
        (handle_unknown_cmd(id_str), None)
    }
}

fn has_cmd(line: &[u8], cmd: &[u8]) -> bool {
    let mut pat: [u8; 64] = [0; 64];
    pat[0..7].copy_from_slice(b"\"cmd\":\"");
    let clen = cmd.len().min(50);
    pat[7..7 + clen].copy_from_slice(&cmd[..clen]);
    pat[7 + clen] = b'"';
    let pat = &pat[..8 + clen];

    let line_len = line.len();
    if line_len < pat.len() {
        return false;
    }
    for i in 0..=line_len - pat.len() {
        if line[i..].starts_with(pat) {
            return true;
        }
    }
    false
}

/// Extract "id" for response
fn copy_id(line: &[u8], out: &mut [u8]) -> usize {
    let prefix = b"\"id\":\"";
    if line.len() < prefix.len() + 1 {
        out[0] = b'0';
        return 1;
    }
    for i in 0..=line.len() - prefix.len() {
        if line[i..].starts_with(prefix) {
            let start = i + prefix.len();
            let mut j = 0;
            while start + j < line.len() && j < out.len() - 1 && line[start + j] != b'"' {
                out[j] = line[start + j];
                j += 1;
            }
            return j;
        }
    }
    out[0] = b'0';
    1
}

/// Accumulate a byte into the line buffer.
/// Returns `true` when a complete line is ready for processing.
/// On overflow the buffer is cleared and the `discarding` flag is set so that
/// the remainder of the oversized frame is silently dropped — preventing a
/// truncated tail from being mistaken for a valid command.
fn accumulate_byte(line_buf: &mut heapless::Vec<u8, 256>, b: u8, discarding: &mut bool) -> bool {
    let is_terminator = b == b'\n' || b == b'\r';

    if *discarding {
        if is_terminator {
            *discarding = false;
            line_buf.clear();
        }
        return false;
    }

    if !is_terminator {
        if line_buf.push(b).is_err() {
            *discarding = true;
            line_buf.clear();
        }
        return false;
    }
    !line_buf.is_empty()
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut config = Config::default();
    config.baudrate = 115_200;

    let mut usart = Uart::new_blocking(p.USART2, p.PA3, p.PA2, config).unwrap();
    let mut led = Output::new(p.PA5, Level::Low, Speed::Low);

    info!("Corvus Nucleo firmware ready on USART2 (115200)");

    let mut line_buf: heapless::Vec<u8, 256> = heapless::Vec::new();
    let mut id_buf = [0u8; 16];
    let mut discarding = false;

    loop {
        let mut byte = [0u8; 1];
        if usart.blocking_read(&mut byte).is_err() {
            continue;
        }

        if !accumulate_byte(&mut line_buf, byte[0], &mut discarding) {
            continue;
        }

        let id_len = copy_id(&line_buf, &mut id_buf);
        let id_str = str::from_utf8(&id_buf[..id_len]).unwrap_or("0");

        let (resp_buf, led_action) = process_command(&line_buf, id_str);

        if let Some((_, value)) = led_action {
            led.set_level(if value != 0 { Level::High } else { Level::Low });
        }

        let _ = usart.blocking_write(resp_buf.as_bytes());
        let _ = usart.blocking_write(b"\n");
        line_buf.clear();
    }
}
