use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use crate::{
    env::{TestEnv, output},
    server::{FakeServer, Reply},
};

const PERSON: [&str; 3] = ["fs", "pessoa", "SINT-001"];

#[test]
fn after_a_429_it_waits_the_retry_after_and_completes_the_request() {
    let calls = AtomicUsize::new(0);
    let fs = FakeServer::start(move |_| {
        if calls.fetch_add(1, Ordering::Relaxed) == 0 {
            Reply::empty(429)
                .with_header("Retry-After", "1")
                .with_header("X-PROCESSING-TIME", "200")
        } else {
            Reply::recorded("person.json")
        }
    });
    let env = TestEnv::new(&fs).with_var("GARIMPO_PAUSA_429_S", "0");
    env.deliver_session();

    let person = env.run(&PERSON);

    assert_eq!(person.code, 0, "{}", person.stderr);
    assert!(person.stdout.starts_with("SINT-001\t"), "{}", person.stdout);
    assert!(person.stderr.contains("429"), "{}", person.stderr);
    let requests = fs.requests();
    assert_eq!(requests.len(), 2);
    let gap = requests[1].arrived - requests[0].arrived;
    assert!(gap >= Duration::from_secs(1), "{gap:?}");
}

#[test]
fn a_403_block_exits_with_its_own_code_and_no_retry() {
    let fs = FakeServer::start(|_| Reply::empty(403));
    let env = TestEnv::logged_in(&fs);

    let person = env.run(&PERSON);

    assert_eq!(person.code, 4);
    assert!(person.stderr.contains("bloqueio"), "{}", person.stderr);
    assert!(
        person
            .stderr
            .contains("não significa que o dado não existe"),
        "{}",
        person.stderr
    );
    assert_eq!(fs.requests().len(), 1);
}

#[test]
fn a_repeating_429_becomes_a_block_after_three_attempts() {
    let fs = FakeServer::start(|_| Reply::empty(429).with_header("Retry-After", "0"));
    let env = TestEnv::new(&fs).with_var("GARIMPO_PAUSA_429_S", "0");
    env.deliver_session();

    let person = env.run(&PERSON);

    assert_eq!(person.code, 4);
    assert_eq!(fs.requests().len(), 3);
}

#[test]
fn a_pause_longer_than_the_max_wait_exits_at_once_and_holds_for_the_next_command() {
    let fs = FakeServer::start(|_| Reply::empty(429).with_header("Retry-After", "1"));
    // The pause after a 429 is long (60 s), even when the server asks for 1 s.
    let env = TestEnv::new(&fs).with_var("GARIMPO_ESPERA_MAX_S", "5");
    env.deliver_session();

    let paused = env.run(&PERSON);
    let after = env.run(&PERSON);

    for command in [&paused, &after] {
        assert_eq!(command.code, 4, "{}", command.stderr);
        let (_, rest) = command.stderr.split_once("próximo passo: espere ").unwrap();
        let left: u64 = rest.split(' ').next().unwrap().parse().unwrap();
        assert!((55..=60).contains(&left), "{}", command.stderr);
    }
    // The second command does not even call the server: the pause was kept.
    assert_eq!(fs.requests().len(), 1);
}

#[test]
fn a_retry_after_outside_429_also_holds_the_next_request() {
    let calls = AtomicUsize::new(0);
    let fs = FakeServer::start(move |_| {
        if calls.fetch_add(1, Ordering::Relaxed) == 0 {
            Reply::empty(503).with_header("Retry-After", "1")
        } else {
            Reply::recorded("person.json")
        }
    });
    let env = TestEnv::logged_in(&fs);

    let unavailable = env.run(&PERSON);
    let after = env.run(&PERSON);

    assert_eq!(unavailable.code, 1, "{}", unavailable.stderr);
    assert_eq!(after.code, 0, "{}", after.stderr);
    let requests = fs.requests();
    let gap = requests[1].arrived - requests[0].arrived;
    assert!(gap >= Duration::from_secs(1), "{gap:?}");
}

#[test]
fn a_redirect_is_reported_and_not_followed_behind_the_regulator() {
    let fs = FakeServer::start(|_| {
        Reply::empty(301).with_header("Location", "/platform/tree/persons/SINT-009")
    });
    let env = TestEnv::logged_in(&fs);

    let merged = env.run(&PERSON);

    assert_eq!(merged.code, 2, "{}", merged.stderr);
    assert!(
        merged.stderr.contains("/platform/tree/persons/SINT-009"),
        "{}",
        merged.stderr
    );
    assert_eq!(fs.requests().len(), 1);
}

#[test]
fn the_reported_processing_time_spends_the_window_budget() {
    let fs = FakeServer::start(|_| {
        Reply::recorded("person.json").with_header("X-PROCESSING-TIME", "600")
    });
    // 1 s of processing every 2 s: two 600 ms requests exceed the budget.
    let env = TestEnv::new(&fs)
        .with_var("GARIMPO_ORCAMENTO_MS", "1000")
        .with_var("GARIMPO_JANELA_S", "2");
    env.deliver_session();

    for _ in 0..3 {
        let person = env.run(&PERSON);
        assert_eq!(person.code, 0, "{}", person.stderr);
    }

    let requests = fs.requests();
    let second = requests[1].arrived - requests[0].arrived;
    let third = requests[2].arrived - requests[0].arrived;
    assert!(second < Duration::from_secs(1), "{second:?}");
    // The third only goes out once the first leaves the 2 s window.
    assert!(third >= Duration::from_secs(2), "{third:?}");
}

#[test]
fn the_session_cap_stops_requests_until_a_new_session() {
    let fs = FakeServer::start(|_| Reply::recorded("person.json"));
    let env = TestEnv::new(&fs).with_var("GARIMPO_TETO_SESSAO", "2");
    env.deliver_session();

    assert_eq!(env.run(&PERSON).code, 0);
    assert_eq!(env.run(&PERSON).code, 0);
    let over_the_cap = env.run(&PERSON);

    assert_eq!(over_the_cap.code, 5);
    assert!(
        over_the_cap.stderr.contains("teto"),
        "{}",
        over_the_cap.stderr
    );
    assert_eq!(fs.requests().len(), 2);

    env.deliver_session();
    assert_eq!(env.run(&PERSON).code, 0);
    assert_eq!(fs.requests().len(), 3);
}

#[test]
fn processes_of_one_account_go_through_the_lock_one_at_a_time() {
    let in_flight = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let fs = FakeServer::start({
        let (in_flight, peak) = (Arc::clone(&in_flight), Arc::clone(&peak));
        move |_| {
            let now = in_flight.fetch_add(1, Ordering::Relaxed) + 1;
            peak.fetch_max(now, Ordering::Relaxed);
            // Plenty of time for another process to arrive, if the lock let it.
            thread::sleep(Duration::from_millis(150));
            in_flight.fetch_sub(1, Ordering::Relaxed);
            Reply::recorded("person.json")
        }
    });
    let env = TestEnv::logged_in(&fs);

    let processes: Vec<_> = (0..4)
        .map(|_| env.command(&PERSON).spawn().unwrap())
        .collect();

    for process in processes {
        let person = output(process);
        assert_eq!(person.code, 0, "{}", person.stderr);
    }
    assert_eq!(fs.requests().len(), 4);
    assert_eq!(peak.load(Ordering::Relaxed), 1);
}
