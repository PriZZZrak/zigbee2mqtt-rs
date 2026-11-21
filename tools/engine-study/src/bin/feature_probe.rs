// Minimal feature probe used by the engine feasibility study.
//
// This is not a runtime. It only answers: can the engine parse and execute
// the language surface that the real upstream converter package uses, and can
// it run Promise/microtask and host-function code?

#[cfg(feature = "boa")]
fn main() {
    use boa_engine::{Context, Source};

    let source = std::fs::read_to_string(
        std::env::args()
            .nth(1)
            .expect("usage: feature-probe <file.js>"),
    )
    .expect("read source");
    let mut context = Context::default();
    let started = std::time::Instant::now();
    let result = context.eval(Source::from_bytes(source.as_bytes()));
    let eval_ms = started.elapsed().as_secs_f64() * 1000.0;
    let jobs_started = std::time::Instant::now();
    let jobs = context.run_jobs();
    let jobs_ms = jobs_started.elapsed().as_secs_f64() * 1000.0;
    match (result, jobs) {
        (Ok(_value), Ok(())) => {
            let report = context.eval(Source::from_bytes("globalThis.__report()"));
            let text = report
                .and_then(|value| value.to_string(&mut context))
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_else(|error| format!("<report error: {error}>"));
            println!("RESULT {text}");
            println!("TIMING eval_ms={eval_ms:.3} jobs_ms={jobs_ms:.3}");
        }
        (Err(error), _) => {
            println!("EVAL_ERROR {error}");
            std::process::exit(2);
        }
        (_, Err(error)) => {
            println!("JOB_ERROR {error}");
            std::process::exit(3);
        }
    }
}

#[cfg(feature = "kataan")]
fn main() {
    use kataan::Interp;
    use kataan::parser::Parser;

    let source = std::fs::read_to_string(
        std::env::args()
            .nth(1)
            .expect("usage: feature-probe <file.js>"),
    )
    .expect("read source");
    let started = std::time::Instant::now();
    let program = match Parser::parse_program(&source) {
        Ok(program) => program,
        Err(error) => {
            println!("PARSE_ERROR {error:?}");
            std::process::exit(2);
        }
    };
    let parse_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut interp = Interp::new();
    let run_started = std::time::Instant::now();
    match interp.run(&program) {
        Ok(_value) => {
            let run_ms = run_started.elapsed().as_secs_f64() * 1000.0;
            let report_program =
                Parser::parse_program("globalThis.__report()").expect("report program");
            let text = match interp.run(&report_program) {
                Ok(value) => interp.realm().to_display_string(value),
                Err(error) => format!("<report error: {error:?}>"),
            };
            println!("RESULT {text}");
            println!("TIMING parse_ms={parse_ms:.3} run_ms={run_ms:.3}");
        }
        Err(error) => {
            println!("RUN_ERROR {error:?}");
            std::process::exit(2);
        }
    }
}

#[cfg(feature = "qjs")]
fn main() {
    use quickjs_oxide::engine::api::core::{Runtime, Value};
    use quickjs_oxide_host::SystemHost;

    let source = std::fs::read_to_string(
        std::env::args()
            .nth(1)
            .expect("usage: feature-probe <file.js>"),
    )
    .expect("read source");
    let runtime = Runtime::new_with_host_services(SystemHost::default());
    let context = runtime.new_context().expect("context");
    let started = std::time::Instant::now();
    match context.eval(&source) {
        Ok(_) => {
            runtime.run_jobs().expect("jobs");
            let report = context.eval("globalThis.__report()").expect("report");
            let Value::String(text) = report else {
                panic!("report not a string")
            };
            println!("RESULT {}", text.to_utf8_lossy());
            println!(
                "TIMING eval_ms={:.3}",
                started.elapsed().as_secs_f64() * 1000.0
            );
        }
        Err(error) => {
            println!("EVAL_ERROR {error:?}");
            std::process::exit(2);
        }
    }
}

#[cfg(not(any(feature = "boa", feature = "kataan", feature = "qjs")))]
fn main() {
    eprintln!("select a feature: --features boa or --features kataan");
    std::process::exit(1);
}
