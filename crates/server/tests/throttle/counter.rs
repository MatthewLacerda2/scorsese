//! The rules, walked through with a made-up clock.

use std::net::IpAddr;

use scorsese_server::accounts::throttle::counter::{
    FIRST_LOCK, FORGIVEN_AFTER, LIMIT, LONGEST_LOCK, WINDOW, lock_for,
};
use scorsese_server::accounts::throttle::{Counter, Key};

const T0: i64 = 1_800_000_000;

/// `counter` after `LIMIT` admitted attempts at `now`.
fn exhausted(counter: &mut Counter, now: i64) {
    for n in 0..LIMIT {
        assert_eq!(counter.attempt(now), Ok(()), "attempt {n}");
    }
}

#[test]
fn ten_attempts_then_a_lock_out_that_ends() {
    let mut counter = Counter::new(T0);
    exhausted(&mut counter, T0);
    assert_eq!(counter.attempt(T0), Err(FIRST_LOCK));
    assert_eq!(counter.attempt(T0 + FIRST_LOCK - 1), Err(1));
    assert_eq!(counter.attempt(T0 + FIRST_LOCK), Ok(()));
}

#[test]
fn a_window_that_passes_starts_the_count_again() {
    let mut counter = Counter::new(T0);
    exhausted(&mut counter, T0);
    assert_eq!(counter.attempt(T0 + WINDOW - 1).map_err(|_| ()), Err(()));

    let mut counter = Counter::new(T0);
    exhausted(&mut counter, T0);
    exhausted(&mut counter, T0 + WINDOW);
}

#[test]
fn each_lock_out_doubles_up_to_a_day() {
    let expected = [15, 30, 60, 120, 240, 480, 960, 1440, 1440];
    for (strikes, minutes) in (1..).zip(expected) {
        assert_eq!(lock_for(strikes), minutes * 60, "strike {strikes}");
    }
    assert_eq!(lock_for(u32::MAX), LONGEST_LOCK);

    let mut counter = Counter::new(T0);
    let mut now = T0;
    for minutes in [15, 30, 60] {
        exhausted(&mut counter, now);
        assert_eq!(counter.attempt(now), Err(minutes * 60));
        now += minutes * 60;
    }
}

#[test]
fn strikes_are_forgiven_after_a_quiet_day_counted_from_the_lock_outs_end() {
    let mut counter = Counter::new(T0);
    counter.strikes = 8;
    exhausted(&mut counter, T0);
    assert_eq!(counter.attempt(T0), Err(LONGEST_LOCK));

    // Right after a day-long lock-out, the next one is as long again.
    let ended = T0 + LONGEST_LOCK;
    let mut again = counter;
    exhausted(&mut again, ended);
    assert_eq!(again.attempt(ended), Err(LONGEST_LOCK));

    // A quiet day after it ended, it starts from the beginning.
    let later = ended + FORGIVEN_AFTER;
    exhausted(&mut counter, later);
    assert_eq!(counter.attempt(later), Err(FIRST_LOCK));
}

#[test]
fn an_address_is_counted_by_what_one_subscriber_holds() {
    let key = |text: &str| Key::address(text.parse::<IpAddr>().unwrap());
    assert_eq!(key("203.0.113.9"), Key::Address("203.0.113.9".into()));
    assert_eq!(key("::ffff:203.0.113.9"), key("203.0.113.9"));
    assert_eq!(key("2001:db8:1:2:aaaa::1"), key("2001:db8:1:2:bbbb::7"));
    assert_eq!(
        key("2001:db8:1:2::1"),
        Key::Address("2001:db8:1:2::/64".into())
    );
    assert_ne!(key("2001:db8:1:2::1"), key("2001:db8:1:3::1"));
}

#[test]
fn the_operator_names_either_kind_of_key() {
    assert_eq!(
        Key::parse("2001:db8:1:2::/64"),
        Key::Address("2001:db8:1:2::/64".into())
    );
    assert_eq!(
        Key::parse(" 203.0.113.9 "),
        Key::Address("203.0.113.9".into())
    );
    assert_eq!(
        Key::parse(" Ana@Example.com "),
        Key::Email("ana@example.com".into())
    );
    assert_eq!(Key::email(&"a".repeat(1000)).text().len(), 254);
}
