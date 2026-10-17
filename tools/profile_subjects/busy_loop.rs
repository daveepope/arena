use std::time::Instant;

const STEP_MILLIS: u64 = 10;

#[inline(never)]
fn foo1() {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < STEP_MILLIS {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn foo2() {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < STEP_MILLIS {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn foo3() {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < STEP_MILLIS {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn foo4() {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < STEP_MILLIS {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn foo5() {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < STEP_MILLIS {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn foo6() {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < STEP_MILLIS {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn foo7() {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < STEP_MILLIS {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn foo8() {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < STEP_MILLIS {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn foo9() {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < STEP_MILLIS {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn bar(millis: u64) {
    let start = Instant::now();
    let mut spinning: u64 = 0;
    while (start.elapsed().as_millis() as u64) < millis {
        spinning = spinning.wrapping_add(1);
    }
    if spinning == u64::MAX {
        unreachable!("spin counter saturated");
    }
}

#[inline(never)]
fn one_pass(total_millis: u64) {
    foo1();
    foo2();
    foo3();
    foo4();
    foo5();
    foo6();
    foo7();
    foo8();
    foo9();
    bar(total_millis.saturating_sub(STEP_MILLIS * 9).max(STEP_MILLIS));
}

fn main() {
    let mut args = std::env::args().skip(1);
    let total_millis: u64 = args
        .next()
        .expect("total work in milliseconds")
        .parse()
        .expect("total work must be an integer number of milliseconds");
    let marker = args.next();
    loop {
        one_pass(total_millis);
        match &marker {
            None => return,
            Some(marker) => std::fs::write(marker, [1]).expect("write marker"),
        }
    }
}
