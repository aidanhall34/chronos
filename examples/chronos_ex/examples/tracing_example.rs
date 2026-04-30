// use tracing::{info, subscriber, Level};

// fn sub_method() {
//     info!("This will be logged to stdout");
// }

// fn main() {
//     // let collector = tracing_subscriber::fmt()
//     //     // filter spans/events with level TRACE or higher.
//     //     .with_max_level(Level::TRACE)
//     //     // build but do not install the subscriber.
//     //     .finish();

//     tracing_subscriber::fmt::init();

//     // subscriber::with_default(collector, || {
//     //     info!("This will be logged to stdout");
//     // });
//     sub_method();
//     info!("This will _not_ be logged to stdout");
// }

use std::{error::Error, io};
use tracing::{debug, error, info, span, warn, Level};

// the `#[tracing::instrument]` attribute creates and enters a span
// every time the instrumented function is called. The span is named after
// the function or method. Parameters passed to the function are recorded as fields.
#[tracing::instrument]
pub fn shave(yak: usize) -> Result<(), Box<dyn Error + 'static>> {
    // this creates an event at the DEBUG level with two fields:
    // - `excitement`, with the key "excitement" and the value "yay!"
    // - `message`, with the key "message" and the value "hello! I'm gonna shave a yak."
    //
    // unlike other fields, `message`'s shorthand initialization is just the string itself.
    debug!(excitement = "yay!", "hello! I'm gonna shave a yak.");
    if yak == 3 {
        warn!("could not locate yak!");
        // note that this is intended to demonstrate `tracing`'s features, not idiomatic
        // error handling! in a library or application, you should consider returning
        // a dedicated `YakError`. libraries like snafu or thiserror make this easy.
        return Err(io::Error::other("shaving yak failed!").into());
    } else {
        debug!("yak shaved successfully");
    }
    Ok(())
}

#[tracing::instrument]
pub fn shave_all(yaks: usize) -> usize {
    // Constructs a new span named "shaving_yaks" at the TRACE level,
    // and a field whose key is "yaks". This is equivalent to writing:
    //
    // let span = span!(Level::TRACE, "shaving_yaks", yaks = yaks);
    //
    // local variables (`yaks`) can be used as field values
    // without an assignment, similar to struct initializers.
    let span = span!(Level::TRACE, "shaving_yaks", yaks);
    let _enter = span.enter();

    info!("shaving yaks");

    let mut yaks_shaved = 0;
    for yak in 1..=yaks {
        let res = shave(yak);
        debug!(yak, shaved = res.is_ok());

        if let Err(ref error) = res {
            // Like spans, events can also use the field initialization shorthand.
            // In this instance, `yak` is the field being initialized.
            error!(yak, error = error.as_ref(), "failed to shave yak!");
        } else {
            yaks_shaved += 1;
        }
        debug!(yaks_shaved);
    }

    yaks_shaved
}

fn main() {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    info!(number_of_yaks, "preparing to shave yaks");

    // let number_shaved = shave_all(number_of_yaks);
    // create a new thread to shave the yaks concurrently.
    let shaving_thread = std::thread::spawn(move || {
        let number_of_yaks = 3;
        info!(number_of_yaks, "preparing to shave yaks");

        let number_shaved = shave_all(number_of_yaks);
        info!(number_shaved, "shaving yaks done");
    });
    let shaving_thread1 = std::thread::spawn(move || {
        let number_of_yaks = 3;
        info!(number_of_yaks, "preparing to shave yaks");

        let number_shaved = shave_all(number_of_yaks);
        info!(number_shaved, "shaving yaks done");
    });

    //join both threads
    shaving_thread.join().unwrap();
    shaving_thread1.join().unwrap();
    // std::thread::spawn(move || {
    //     let number_shaved = shave_all(number_of_yaks);
    //     debug!(all_yaks_shaved = number_shaved == number_of_yaks);
    // });
    // info!(all_yaks_shaved = number_shaved == number_of_yaks, "yak shaving completed.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shave() {
        let res = shave(3);
        assert!(res.is_ok());
    }

    #[test]
    fn test_shave_all() {
        let res = shave_all(3);
        assert_eq!(res, 2);
    }
}
