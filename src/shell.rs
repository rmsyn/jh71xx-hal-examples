#![no_std]
#![no_main]

extern crate panic_halt;

use core::fmt::Write;

use embedded_io::Write as IoWrite;
use riscv_rt::entry;
use ushell::{autocomplete::*, history::*, *};

use jh71xx_hal::{gpio, pac, uart::{self, Uart}};

const CMD_LEN: usize = 16;
const HISTORY_SIZE: usize = 4;
const COMMANDS: usize = 2;

const HELP: &str = "\r\n\
\x1b[31mL\x1b[32mE\x1b[34mD\x1b[33m Shell\x1b[0m\r\n\r\n\
USAGE:\r\n\
\x20 command [arg]\r\n\r\n\
COMMANDS:\r\n\
\x20 clear     Clear screen\r\n\
\x20 help      Print this message\r\n
";

type Serial<'d> = Uart<'d, pac::uart0::RegisterBlock, pac::UART0>;
type Autocomplete = StaticAutocomplete<COMMANDS>;
type History = LRUHistory<CMD_LEN, HISTORY_SIZE>;
type Shell<'d> = UShell<Serial<'d>, Autocomplete, History, CMD_LEN>;

pub struct Env;

impl Env {
    /// Creates a new [Env].
    pub const fn new() -> Self {
        Self {}
    }
}

impl<'d> Environment<Serial<'d>, Autocomplete, History, (), CMD_LEN> for Env {
    fn command(&mut self, shell: &mut Shell<'d>, cmd: &str, _args: &str) -> ushell::SpinResult<()> {
        match cmd {
            "clear" => shell.clear()?,
            "help" => shell.write_str(HELP)?,
            _ => shell.write_str(HELP)?,
        }
        Ok(())
    }

    fn control(&mut self, _shell: &mut Shell, _code: u8) -> ushell::SpinResult<()> {
        Ok(())
    }
}

/// PLL frequency configuration struct.
///
/// Represents common fields for configuring PLL clocks on JH7110 SoCs.
///
/// From the `oreboot` startup code.
pub struct PllFreq {
    pub prediv: u8,
    pub fbdiv: u16,
    pub postdiv1: u8,
    pub dacpd: bool,
    pub dsmpd: bool,
}

pub const PLL0_1_000_000_000: PllFreq = PllFreq {
    prediv: 3,
    fbdiv: 125,
    postdiv1: 1,
    dacpd: true,
    dsmpd: true,
};

pub const PLL2_1_188_000_000: PllFreq = PllFreq {
    prediv: 3,
    fbdiv: 99,
    postdiv1: 1,
    dacpd: true,
    dsmpd: true,
};

/// Sets the frequency of the PLL0 system clock.
///
/// Based on the `oreboot` startup implementation.
pub fn set_pll0_freq(syscon: &pac::SYS_SYSCON, f: PllFreq) {
    syscon.sys_syscfg_8().modify(|_, w| {
        w.pll0_pd().set_bit()
    });

    syscon.sys_syscfg_6().modify(|_, w| {
        w.pll0_dacpd().variant(f.dacpd)
         .pll0_dsmpd().variant(f.dsmpd)
    }); 

    syscon.sys_syscfg_9().modify(|_, w| {
        w.pll0_prediv().variant(f.prediv)
    });

    syscon.sys_syscfg_7().modify(|_, w| {
        w.u0_pll_wrap_pll0_fbdiv().variant(f.fbdiv)
    });

    // NOTE: Not sure why, but the original code does this shift, and defines
    // all postdiv values for all PLLs and config to be 1, effectively dropping
    // to 0 here.
    syscon.sys_syscfg_8().modify(|_, w| {
        w.pll0_postdiv1().variant(f.postdiv1 >> 1)
         .pll0_pd().clear_bit()
    });
}

/// Sets the frequency of the PLL2 system clock.
///
/// Based on the `oreboot` startup implementation.
pub fn set_pll2_freq(syscon: &pac::SYS_SYSCON, f: PllFreq) {
    syscon.sys_syscfg_12().modify(|_, w| {
        w.pll2_pd().set_bit()
    });

    syscon.sys_syscfg_11().modify(|_, w| {
        w.pll2_dacpd().variant(f.dacpd)
         .pll2_dsmpd().variant(f.dsmpd)
    }); 

    syscon.sys_syscfg_13().modify(|_, w| {
        w.pll2_prediv().variant(f.prediv)
    });

    syscon.sys_syscfg_11().modify(|_, w| {
        w.pll2_fbdiv().variant(f.fbdiv)
    });

    // NOTE: Not sure why, but the original code does this shift, and defines
    // all postdiv values for all PLLs and config to be 1, effectively dropping
    // to 0 here.
    syscon.sys_syscfg_12().modify(|_, w| {
        w.pll2_postdiv1().variant(f.postdiv1 >> 1)
         .pll2_pd().clear_bit()
    });
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // set the pll0 clock frequency
    set_pll0_freq(&dp.SYS_SYSCON, PLL0_1_000_000_000);
    set_pll2_freq(&dp.SYS_SYSCON, PLL2_1_188_000_000);

    // set pll0 in the cpu clock mux selector
    dp.SYSCRG.clk_cpu_root().modify(|_, w| w.clk_mux_sel().variant(0));
    // set pll2 in the bus clock mux selector
    dp.SYSCRG.clk_bus_root().modify(|_, w| w.clk_mux_sel().variant(1));
    // set pll2 in the peripheral clock mux selector
    dp.SYSCRG.clk_peripheral_root().modify(|_, w| w.clk_mux_sel().variant(1));
    // set always-on APB clock mux selector to clk_osc
    dp.AONCRG.clk_aon_apb().modify(|_, w| w.clk_mux_sel().variant(1));

    // set GPIO to 3.3v
    dp.SYS_SYSCON.sys_syscfg_3().modify(|_, w| {
        w.scfg_vout0_remap_awaddr_gpio0().clear_bit()
         .scfg_vout0_remap_awaddr_gpio1().clear_bit()
         .scfg_vout0_remap_awaddr_gpio2().clear_bit()
         .scfg_vout0_remap_awaddr_gpio3().clear_bit()
    });

    dp.SYS_PINCTRL.ioirq_0().modify(|_, w| w.gpen_0().set_bit());
    dp.SYS_PINCTRL.gpio_40().modify(|_, w| w.ie().clear_bit().pd().set_bit().pu().set_bit());
    //blink(&dp.SYS_PINCTRL, 10);
    dp.PLIC.enable_0_0().modify(|r, w| {
        let val = r.bits();
        // SAFETY: need to rewrite the SVD to expose the `Resettable` API
        unsafe { w.bits(val | (1 << pac::Interrupt::UART0 as u32)) }
    });

    // Configure TXD as a push-pull output
    dp.SYS_PINCTRL.gpo_doen_1().modify(|_, w| w.doen_5().variant(0b01));
    // Set u0 UART TXD to GPIO5 (GPO signal index 79)
    dp.SYS_PINCTRL.gpo_dout_4_7().modify(|_, w| w.dout_5().variant(gpio::GpoFunction::U0_DW_UART_SOUT));

    // Configure RXD as pull-up input
    dp.SYS_PINCTRL.gpo_doen_1().modify(|_, w| w.doen_6().variant(0b10));
    // Set u0 UART RXD to GPIO6 (6 + 2 to configure GPI signal)
    dp.SYS_PINCTRL.gpi_3().modify(|_, w| w.uart_sin_0().variant(gpio::PAD_GPIO6 as u8 + 2));

    // Configure JTAG TRST as pull-up input
    dp.SYS_PINCTRL.gpo_doen_8().modify(|_, w| w.doen_35().variant(0b10));
    dp.SYS_PINCTRL.gpi_1().modify(|_, w| w.jtag_trstn().variant(gpio::PAD_GPIO36 as u8 + 2));

    // Configure JTAG TDI as pull-up input
    dp.SYS_PINCTRL.gpo_doen_15().modify(|_, w| w.doen_61().variant(0b10));
    dp.SYS_PINCTRL.gpi_4().modify(|_, w| w.jtag_tdi().variant(gpio::PAD_GPIO61 as u8 + 2));

    // Configure JTAG TMS as pull-up input
    dp.SYS_PINCTRL.gpo_doen_15().modify(|_, w| w.doen_63().variant(0b10));
    dp.SYS_PINCTRL.gpi_5().modify(|_, w| w.jtag_tms().variant(gpio::PAD_GPIO63 as u8 + 2));

    // Configure JTAG TCLK as pull-up input
    dp.SYS_PINCTRL.gpo_doen_15().modify(|_, w| w.doen_60().variant(0b10));
    dp.SYS_PINCTRL.gpi_7().modify(|_, w| w.jtag_tck().variant(gpio::PAD_GPIO60 as u8 + 2));

    // Configure JTAG TDO as floating output
    dp.SYS_PINCTRL.gpo_doen_11().modify(|_, w| w.doen_44().variant(0b1000));
    dp.SYS_PINCTRL.gpo_dout_44_47().modify(|_, w| w.dout_44().variant(gpio::GpoFunction::U0_JTAG_CERTIFICATION_TDO));

    let config = uart::Config {
        data_len: uart::DataLength::Eight,
        stop: uart::Stop::One,
        parity: uart::Parity::None,
        baud_rate: uart::BaudRate::B115200,
        clk_hz: 1_000_000_000,
        //clk_hz: 51_200_000,
        //clk_hz: 24_000_000,
        //clk_hz: 12_800_000,
    };

    let mut uart = Uart::new_with_config(dp.UART0, uart::TIMEOUT_US, config);

    uart.write(b"Hello from VisionFive2!\n").ok();

    // enable the APB0 clock
    dp.SYSCRG.clk_apb0().modify(|_, w| w.clk_icg().clear_bit().clk_icg().set_bit());
    // enable u0 UART APB clock
    dp.SYSCRG.clk_u0_uart_apb().modify(|_, w| w.clk_icg().clear_bit().clk_icg().set_bit());

    let autocomplete = StaticAutocomplete(["clear", "help"]);
    let history = LRUHistory::default();
    let mut env = Env::new();

    let mut shell = UShell::new(uart, autocomplete, history);

    loop {
        shell.spin::<(), Env>(&mut env).ok();
    }
}

pac::interrupt!(UART0, uart0);

fn uart0() {
    // UART0 interrupt handler is running in an interrupt-free context,
    // and should thus have exclusive access to peripheral memory.
    let pinctrl = unsafe { &*pac::SYS_PINCTRL::ptr() };

    for _ in 0..100 {
        for _ in 0..1_500_000 {}
        pinctrl.gpo_dout_40_43().modify(|_, w| w.dout_40().variant(0b10));
        for _ in 0..1_500_000 {}
        pinctrl.gpo_dout_40_43().modify(|_, w| w.dout_40().variant(0b00));
    }
}
