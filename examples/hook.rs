use irisia::hook::{Effect, ProviderObject, Signal, ToProviderObject};
use std::{fmt::Write, rc::Rc, time::Duration};
use tokio::{select, sync::Notify};

#[tokio::main]
async fn main() {
    let local_set = tokio::task::LocalSet::new();
    let _guard = local_set.enter();

    let text = Signal::state("apple".to_string());
    let factor = Signal::state(1);
    let number = self_increment(factor.to_object());

    let sentence = Signal::builder(String::new())
        .dep(
            |mut setter, (t, n)| {
                setter.clear();
                write!(&mut *setter, "I have {n} {t}").unwrap();
            },
            (text.clone(), number.clone()),
        )
        .build();

    let trailing_s = Signal::memo(|count| if *count < 2 { "." } else { "s." }, number.clone());
    let final_sentence = Signal::memo(|(s1, s2)| format!("{s1}{s2}"), (sentence, trailing_s));

    let stop = Rc::new(Notify::new());

    let _consumer = Signal::builder(stop.clone())
        .dep(
            |_, final_sentence| {
                println!("sentence changed: {}", &*final_sentence);
            },
            final_sentence,
        )
        .dep(
            move |stop, number| {
                if *number % 3 == 0 {
                    let mut w = factor.write();
                    *w += 1;
                    println!("increase factor to {}", *w);
                } else if *number > 15 {
                    println!("stop");
                    stop.notify_one();
                }
            },
            number,
        )
        .build();

    select! {
        _ = stop.notified() => {}
        _ = local_set => {}
    }
}

fn self_increment(factor: ProviderObject<u32>) -> Effect<u32> {
    Effect::new(
        0u32,
        |state, factor| {
            let factor = *factor;
            let handle = tokio::task::spawn_local(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                interval.tick().await;

                loop {
                    interval.tick().await;
                    let continue_ = state.update(|mut n| {
                        *n += factor;
                        true
                    });
                    if continue_ != Some(true) {
                        break;
                    }
                }
            });

            move || {
                handle.abort();
            }
        },
        factor,
    )
}
