// Runaway-script guard probe: loop limits for Boa, host interrupt for Kataan.
// Not a runtime; used to record whether a host can bound infinite JavaScript.

#[cfg(feature = "boa")]
fn main() {
    use boa_engine::{Context, Source};
    use std::time::Instant;

    let mut context = Context::default();
    context
        .runtime_limits_mut()
        .set_loop_iteration_limit(10_000_000);
    context.runtime_limits_mut().set_recursion_limit(512);
    let started = Instant::now();
    let result = context.eval(Source::from_bytes("while (true) {}"));
    println!(
        "boa loop_limit error={:?} ms={:.1}",
        result.err().map(|e| e.to_string()),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let result = context.eval(Source::from_bytes("(function f(){ return f(); })()"));
    println!(
        "boa recursion_limit error={:?} ms={:.1}",
        result.err().map(|e| e.to_string()),
        started.elapsed().as_secs_f64() * 1000.0
    );

    // Long-running builtin: the loop limit cannot interrupt it.
    let started = Instant::now();
    let result = context.eval(Source::from_bytes(
        "(() => { const s = 'a'.repeat(200000); let n = 0; for (let i = 0; i < 2000; i++) n += s.replace(/a/g, 'b').length; return n; })()",
    ));
    println!(
        "boa long_builtin ok={} ms={:.1}",
        result.is_ok(),
        started.elapsed().as_secs_f64() * 1000.0
    );
}

#[cfg(feature = "kataan")]
fn main() {
    use kataan::interrupt::Interrupt;
    use kataan::parser::Parser;
    use std::time::{Duration, Instant};

    let source = "while (true) {}";
    let program = Parser::parse_program(source).expect("parse");
    let mut interp = kataan::Interp::new();
    let interrupt = Interrupt::new();
    interp.realm_mut().interrupt = Some(interrupt.clone());
    let watchdog = interrupt.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        watchdog.trip();
    });
    let started = Instant::now();
    let result = interp.run(&program);
    println!(
        "kataan interrupt error={:?} ms={:.1}",
        result.err(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    // The same handle can be reused after an explicit clear/arm.
    interrupt.clear();
    let watchdog = interrupt.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        watchdog.trip();
    });
    let started = Instant::now();
    let program = Parser::parse_program("for (;;) {}").expect("parse");
    let result = interp.run(&program);
    println!(
        "kataan interrupt_reuse error={:?} ms={:.1}",
        result.err(),
        started.elapsed().as_secs_f64() * 1000.0
    );
}

#[cfg(feature = "qjs")]
fn main() {
    use quickjs_oxide::engine::api::core::{HeapLimits, Runtime, RuntimeOptions};
    use std::time::{Duration, Instant};

    let runtime = Runtime::new();
    let context = runtime.context();
    let handle = runtime.interrupt_handle();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        handle.interrupt();
    });
    let started = Instant::now();
    let result = context.eval("while (true) {}");
    println!(
        "qjs interrupt error={:?} ms={:.1}",
        result.err(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    runtime.interrupt_handle().clear();
    println!("qjs after_interrupt ok={:?}", context.eval("1 + 1"));

    let runtime = Runtime::with_options(RuntimeOptions::from(HeapLimits {
        max_bytes: 16 << 20,
    }));
    let context = runtime.context();
    let started = Instant::now();
    let result = context.eval(
        "try { const a = []; while (true) a.push(new Array(100000).fill(0)); } catch (e) { e.name + ': ' + e.message }",
    );
    println!(
        "qjs heap_limit result={:?} ms={:.1}",
        result,
        started.elapsed().as_secs_f64() * 1000.0
    );
    println!("qjs after_heap_limit ok={:?}", context.eval("1 + 1"));
}

#[cfg(not(any(feature = "boa", feature = "kataan", feature = "qjs")))]
fn main() {
    eprintln!("select --features boa or --features kataan");
    std::process::exit(1);
}
