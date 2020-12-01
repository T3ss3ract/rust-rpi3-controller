extern crate sysfs_gpio;
extern crate semver;

use std::env;
use std::str;
use sysfs_gpio::Direction;
use sysfs_gpio::Pin;
use std::thread;
use std::io::{stdout, Write, Read};
use std::time::Duration;
use std::thread::sleep;
use serde::{Serialize, Deserialize};
use oorandom;
use semver::Version;
use curl::easy::Easy;
use daemonize::Daemonize;
use std::fs::File;
use std::sync::mpsc;

// ----------------------------------------------- FUNCTIONS

//error trace message
pub fn error(message: &str)
{
    println!(" \x1B[1;31m[ERROR] :\x1B[0m \x1B[31m{}\x1B[0m", message);
}
//info message (green)
pub fn info(message: &str)
{
    println!("  \x1B[1;32m[INFO] :\x1B[0m \x1B[32m{}\x1B[0m", message);
}
//warn message (yellow)
pub fn warn(message: &str)
{
    println!("  \x1B[1;33m[WARN] :\x1B[0m \x1B[33m{}\x1B[0m", message);
}
pub fn wait(message: &str)
{
    println!("  \x1B[1;34m[WAIT] :\x1B[0m \x1B[34m{}\x1B[0m", message);
}
pub fn critical(message: &str)
{
    println!("  \x1B[1;35m[CRITICAL] :\x1B[0m \x1B[35m{}\x1B[0m", message);
}
//outputs a command
pub fn command(message: &str)
{
    println!("  \x1B[1;36m[CMD] :\x1B[0m \x1B[36m{}\x1B[0m", message);
}

//vnet purple print
pub fn vnet(message: &str)
{
    println!("  \x1B[1;35m[CORE] :\x1B[0m \x1B[35m{}\x1B[0m", message);
}

//use this for all cli prints longer than 1 line
pub fn vnet_longstr(message: &str)
{
    println!("  \x1B[1;35mCORE :\x1B[0m \x1B[35m\n\n========================================================================\n{}\n========================================================================\x1B[0m", message);
}

// outputs a link
pub fn link(message: &str) {
    println!("  \x1B[1;34mLINK :\x1B[0m \x1B[4;34m{}\x1B[0m", message);
}

//misc message
pub fn misc(message: &str) {
    println!("  \x1B[2;38mMISC :\x1B[2;38m{}\x1B[0m", message);
}



struct Arguments
{
    pin: u64,
    duration_ms: u64,
    period_ms: u64,
}


//blinks the led at pin for a duration with period led lit
fn blink_led(led: u64, duration_ms: u64, period: u64) -> sysfs_gpio::Result<()>
{
    let mled = Pin::new(led);
    mled.with_exported(|| {
        mled.set_direction(Direction::Low)?;
        let iterations = duration_ms / period /2;
        for _ in 0..iterations
        {
            mled.set_value(0)?;
            sleep(Duration::from_millis(period));
            mled.set_value(1)?;
            sleep(Duration::from_millis(period));
        }
        mled.set_value(0)?;
        Ok(())
    })
}

fn print_usage()
{
    println!("Usage: ./blink <pin> <duration> <period>");
}


//get arguments from command line
fn get_args() -> Option<Arguments>
{
    let args: Vec<String> = env::args().collect();
    if args.len() != 4  //check for exactly 4 command line args{
    {
        return None;
    }
    let pin = match args[1].parse::<u64>()
    {
        Ok(pin) => pin,
        Err(_) => return None,
    };
    let duration_ms = match args[2].parse::<u64>()
    {
        Ok(ms) => ms,
        Err(_) => return None,
    };
    let period_ms = match args[3].parse::<u64>()
    {
        Ok(ms) => ms,
        Err(_) => return None,
    };
    Some(Arguments {
        pin: pin,
        duration_ms: duration_ms,
        period_ms: period_ms,
    })

}

fn rust_check()
{
    match get_args()
    {
        None => print_usage(),
        Some(args) => match blink_led(args.pin, args.duration_ms, args.period_ms)
        {
            Ok(()) => println!("Success"),
            Err(err) => println!("Problem: {}", err),
        },
    }
}


fn poll_input(pin_num: u64) -> sysfs_gpio::Result<()>
{

    // NOTE: this currently runs forever and as such if
    // the app is stopped (Ctrl-C), no cleanup will happen
    // and the GPIO will be left exported.  Not much
    // can be done about this as Rust signal handling isn't
    // really present at the moment.  Revisit later.
    let input = Pin::new(pin_num);
    input.with_exported(|| {
        input.set_direction(Direction::In)?;
        let mut prev_val: u8 = 255;
        loop
        {
            let val = input.get_value()?;
            if val != prev_val
            {
                println!("Pin state: {}", if val == 0 { "Low" } else { "High" });
                prev_val = val;
            }
            sleep(Duration::from_millis(10));
        }
    })
}

fn poll_test()
{
    let args: Vec<String> = env::args().collect();
    if args.len() != 2
    {
        println!("Usage ./poll <pin>");
    }
    else
    {
        match args[1].parse::<u64>()
        {
            Ok(pin) => match poll_input(pin)
            {
                Ok(()) => println!("Polling complete"),
                Err(err) => println!("Error: {}", err),
            },
            Err(_) => println!("Usage: ./poll <pin>"),
        }
    }
}

//http post example from curl crate docs
fn http_post_curl(addr: &str, body: &str)
{
    let mut data = body.as_bytes();
    let mut easy = Easy::new();
    easy.url(addr).unwrap();
    easy.post(true).unwrap();
    easy.post_field_size(data.len() as u64).unwrap();

    let mut transfer = easy.transfer();
    transfer.read_function(|buf| {
        Ok(data.read(buf).unwrap_or(0))
    }).unwrap();
    transfer.perform().unwrap();

}



fn fast_curl(url: &str)
{
    let mut easy = Easy::new();
    easy.url(url);
    easy.write_function(|data| {
        stdout().write_all(data).unwrap();
        Ok(data.len())
    }).unwrap();
    easy.perform().unwrap();
}


// ------------------------------------ MAIN

fn main()
{
    let version: &str = "1.0.1";
    assert!(Version::parse(version) == Ok(Version{
        major: 1,
        minor: 0,
        patch: 1,
        pre: vec!(),
        build: vec!(),
    }));

    info(version);
    thread::sleep(Duration::from_secs(2));
    //println!("Blinking LED on a {}.", DeviceInfo::new()?.model());

    info("starting process...");
    command("Device: Raspberry Pi");
    //misc(&rng.rand_float().to_string());

    fast_curl("https://www.google.com/");

    //do basic curl here, works as intended
    //returns fat blob of html
    /*
    let mut easy = Easy::new();
    easy.url("https://www.google.com/").unwrap();
    easy.write_function(|data| {
        stdout().write_all(data).unwrap();
        Ok(data.len())  //no semicolon on Ok() lines
    }).unwrap();
    easy.perform().unwrap();
     */
    vnet("\nTrying to read from buffer.........");
    thread::sleep(Duration::from_secs(2));

    //read web data into vector;
    let mut dst = Vec::new();
    let mut nega = Easy::new();

    nega.url("https://www.google.com/").unwrap();
    {
        let mut transfer = nega.transfer();
        transfer.write_function(|data| {
            dst.extend_from_slice(data);
            //stdout().write_all(data).unwrap();
            Ok(data.len())
        }).unwrap();
        transfer.perform().unwrap();
    }
    //println!("{}", str::from_utf8(&*dst).unwrap());

    //vnet("CHECKING RETURNS::::");
    //println!("{}", s);
    //thread::sleep(Duration::from_secs(2));
    //info(issue_l.scheme());
    //info(issue_l.host_str().unwrap());

    /*

    let stdout = File::create("/tmp/nsxvdaemon.out").unwrap();
    let stderr = File::create("/tmp/nsxvdaemon.err").unwrap();

    let dm = Daemonize::new()
        .pid_file("/tmp/nsxv.pid")
        .chown_pid_file(true)
        .working_directory("/tmp")
        .user("nobody")
        .group("daemon")
        .group(2)
        .umask(0o777)
        .stdout(stdout)
        .stderr(stderr)
        .exit_action(|| println!("see this before master process exits"))
        .privileged_action(|| "Executed before drop privileges");

    match dm.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }

    loop {
        println!("daemon test work good?");
    }

     */


    info("starting pin proceses (threaded)");
    thread::sleep(Duration::from_millis(2000));




    //pin control code is embedded in a loop

    let handle1 = thread::spawn(move ||{
        let seed = 2435;
        let mut rng = oorandom::Rand32::new(seed);
        let my_led = Pin::new(22);
        loop
        {
            command("THREAD 1");
            my_led.with_exported(|| {
                my_led.set_direction(Direction::Out).unwrap();
                //led2.set_direction(Direction::Out).unwrap();

                vnet("pin 22 on");
                my_led.set_value(0).unwrap();
                thread::sleep(Duration::from_millis(200));
                wait(&rng.rand_u32().to_string());
                info("pin 22 off");
                my_led.set_value(1).unwrap();
                thread::sleep(Duration::from_millis(200));
                info("exiting process.");
                Ok(())
            }).unwrap();
        }
    });




    let handle2 = thread::spawn(move || {
        let seed = 101025;
        let mut rng = oorandom::Rand32::new(seed);
        let led2: Pin = Pin::new(23);
        loop
        {
            command("THREAD 2");
            led2.with_exported(|| {
                led2.set_direction(Direction::Out).unwrap();
                //led2.set_direction(Direction::Out).unwrap();
                vnet("pin 23  on");
                led2.set_value(0).unwrap();
                thread::sleep(Duration::from_millis(200));
                wait(&rng.rand_u32().to_string());
                info("pin 23 off");
                //my_led.set_value(1).unwrap();
                led2.set_value(1).unwrap();
                thread::sleep(Duration::from_millis(200));

                info("exiting process.");
                Ok(())
            }).unwrap();
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();






}
