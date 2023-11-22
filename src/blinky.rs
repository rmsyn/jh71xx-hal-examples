#![no_std]
#![no_main]

extern crate panic_halt;

use riscv_rt::entry;
use jh71xx_hal::pac;

#[entry]
fn main() -> ! {
    let pinctrl = unsafe { &*pac::SYS_PINCTRL::ptr() };

    pinctrl.ioirq_0().modify(|_, w| w.gpen_0().set_bit());

    pinctrl.gpo_doen_10().modify(|_, w| w.doen_40().variant(0b1));

    loop {
        for _ in 0..750_000 {
            pinctrl.gpo_dout_40_43().modify(|_, w| w.dout_40().variant(0b01));
        }
        for _ in 0..750_000 {
            pinctrl.gpo_dout_40_43().modify(|_, w| w.dout_40().variant(0b00));
        }
    }
}
