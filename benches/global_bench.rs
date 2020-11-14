use criterion::Criterion;
use criterion::{criterion_group, criterion_main};
use metrics_rs::{cpu::*, disks::*, miscs::*, users::*};

fn all_gather() {
    let _host_info = match get_host_info() {
        Ok(val) => val,
        Err(_) => return,
    };

    {
        let _uuid = get_uuid().expect("Cannot retrieve UUID");
        let _cpu_freq = get_avg_cpufreq();
        //let _sensors = get_sensors_data();
        let _disks = get_disks_data();
        let _iostats = match get_iostats() {
            Ok(val) => Some(val),
            Err(_) => None,
        };
        let _users = get_users();
    };
}

pub fn global_benche(c: &mut Criterion) {
    c.bench_function("all_gather", |b| b.iter(|| all_gather()));
}

criterion_group!(benches, global_benche);
criterion_main!(benches);
