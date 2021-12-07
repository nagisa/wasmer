use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use wasmer::*;

fn call_many_functions(n: usize) -> Vec<u8> {
    let fndefs = (0..n)
        .map(|idx| format!(r#"(func $fn{idx} return)"#, idx = idx))
        .collect::<String>();
    let calls = (0..n)
        .map(|idx| format!("call $fn{idx}\n", idx = idx))
        .collect::<String>();
    let wat = format!(
        r#"(module
            {fndefs}
            (func (export "main") {calls} return)
            (func (export "single") call $fn0 return))"#,
        fndefs = fndefs,
        calls = calls
    );
    wat2wasm(wat.as_bytes()).expect("wat must be valid here").to_vec()
}

fn nops(c: &mut Criterion) {
    for size in [1, 10, 100, 1000, 10000] {
        let wasm = call_many_functions(size);
        let store = Store::new(&Universal::new(Singlepass::new()).engine());
        let mut compile = c.benchmark_group("compile");
        compile.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let module = Module::new(&store, &wasm).unwrap();
                let imports = imports! {};
                let _ = Instance::new(&module, &imports).unwrap();
            })
        });
        drop(compile);
        let module = Module::new(&store, &wasm).unwrap();
        let imports = imports! {};
        let instance = Instance::new(&module, &imports).unwrap();
        let mut get_main = c.benchmark_group("get_main");
        get_main.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let _: &Function = instance.exports.get("main").unwrap();
            })
        });
        drop(get_main);
        let main: &Function = instance.exports.get("main").unwrap();
        let mut call_main = c.benchmark_group("call_main");
        call_main.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                black_box(main.call(&[]).unwrap());
            })
        });
        drop(call_main);

        let single: &Function = instance.exports.get("single").unwrap();
        let mut call_single = c.benchmark_group("call_single");
        call_single.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                black_box(single.call(&[]).unwrap());
            })
        });
    }
}

fn serialization(c: &mut Criterion) {
    for size in [1, 10, 100, 1000, 10000] {
        let wasm = call_many_functions(size);
        let universal = Universal::new(Singlepass::new());
        let engine = universal.engine();
        let tunables = BaseTunables::for_target(engine.target());
        let artifact = engine.compile(&wasm, &tunables).unwrap();
        let mut serialize = c.benchmark_group("serialize");
        serialize.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| artifact.serialize())
        });
        let serialized = artifact.serialize().unwrap();
        drop(serialize);

        let mut deserialize = c.benchmark_group("deserialize");
        deserialize.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| unsafe {
                UniversalArtifact::deserialize(&engine, &serialized).unwrap();
            })
        });
        let deserialized = unsafe {
            UniversalArtifact::deserialize(&engine, &serialized).unwrap()
        };
        drop(deserialize);
    }
}

criterion_group!(benches, nops, serialization);

criterion_main!(benches);
