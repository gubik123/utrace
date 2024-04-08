#![no_std]
#![no_main]
use embassy_stm32::{pac, peripherals};
use panic_semihosting as _;
use rtic::app;

use utrace_rtt;
use utrace_rtt::rtt_target;

use rtic_monotonics::{stm32::Tim15, Monotonic};

#[utrace::timestamp]
fn utrace_timestamp_fn() -> u64 {
    (Tim15::now() - <Tim15 as Monotonic>::ZERO).to_micros()
}

#[app(device = pac, peripherals = false, dispatchers = [SAI1, SAI2])]
mod app {

    use core::fmt::Write;

    use embassy_stm32::rcc::{ClockSrc, PllConfig, PllSource, Plldiv, Pllm, Plln};
    use rtic_monotonics::Monotonic;

    use super::*;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(_cx: init::Context) -> (Shared, Local) {
        let mut config = embassy_stm32::Config::default();
        config.rcc.voltage_range = embassy_stm32::rcc::VoltageScale::RANGE1;
        config.rcc.mux = ClockSrc::PLL1_R(PllConfig {
            source: PllSource::HSI, //HSE(Hertz(16_000_000)),
            m: Pllm::DIV1,
            n: Plln::MUL16,
            p: Plldiv::DIV2,
            q: Plldiv::DIV2,
            r: Plldiv::DIV4,
        });

        pac::ICACHE.cr().write(|w| {
            w.set_en(true);
            w.set_waysel(pac::icache::vals::Waysel::NWAYSETASSOCIATIVE);
        });
        let p = embassy_stm32::init(config);

        let channels = rtt_target::rtt_init! {
            up: {
                0: {
                    size: 1024,
                    mode: rtt_target::ChannelMode::NoBlockSkip,
                    name: "Terminal",
                }
            }
        };
        let mut tracing_rtt_channel: rtt_target::UpChannel = channels.up.0;

        // tracing_rtt_channel.write_str("Goose");

        utrace_rtt::init(tracing_rtt_channel);
        // utrace_rtt::write("Second goose".as_bytes());
        {
            // utrace::trace_here!();
        }

        let tim3_input_frequency =
            <peripherals::TIM15 as embassy_stm32::rcc::low_level::RccPeripheral>::frequency().0;
        let timer_token = rtic_monotonics::create_stm32_tim15_monotonic_token!();
        Tim15::start(tim3_input_frequency, timer_token);

        let _ = task1::spawn();
        let _ = task2::spawn();

        (Shared {}, Local {})
    }

    #[utrace::trace]
    async fn task1_sub() {
        Tim15::delay(<Tim15 as Monotonic>::Duration::millis(1)).await;
    }

    #[utrace::trace]
    async fn task2_sub() {
        Tim15::delay(<Tim15 as Monotonic>::Duration::millis(1)).await;
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        // utrace::trace_here!();
        loop {
            // utrace::trace_here!();
            cortex_m::asm::wfi();
        }
    }

    #[task(priority = 1)]
    async fn task1(_: task1::Context) {
        loop {
            Tim15::delay(<Tim15 as Monotonic>::Duration::millis(10)).await;
            task1_sub().await;
        }
    }

    #[task(priority = 2)]
    async fn task2(_: task2::Context) {
        loop {
            Tim15::delay(<Tim15 as Monotonic>::Duration::millis(11)).await;
            task2_sub().await;
        }
    }
}
