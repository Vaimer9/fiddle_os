use x86_64::structures::idt::{ InterruptDescriptorTable, InterruptStackFrame };
use crate::{println, text};

use lazy_static::lazy_static;

use crate::gdt;

use pic8259::ChainedPics;


#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
		Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });



lazy_static! {
	static ref IDT: InterruptDescriptorTable = { 
		let mut idt = InterruptDescriptorTable::new();

		idt.breakpoint.set_handler_fn(breakpoint_handler);
		idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_handler);
		idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_handler);

		unsafe {
			idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
		}

		idt
	};
}

use crate::print;

pub fn init_idt() {
	IDT.load();
}

use pc_keyboard::Keyboard;
use pc_keyboard::ScancodeSet1;
use pc_keyboard::DecodedKey;
use pc_keyboard::layouts;
use pc_keyboard::HandleControl;

use spin::Mutex;

extern "x86-interrupt" fn keyboard_handler(
		stack_frame: InterruptStackFrame
) {
	use x86_64::instructions::port::Port;

	let mut port = Port::new(0x60);
	let scancode: u8 = unsafe { port.read() };


	lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1,
                HandleControl::Ignore)
            );
  }

	let mut keyboard = KEYBOARD.lock();

	if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }



	unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
	}
}

extern "x86-interrupt" fn timer_handler(
		stack_frame: InterruptStackFrame
) {
	unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
  }
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: InterruptStackFrame
	)
{
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
	stack_frame: InterruptStackFrame, _error_code: u64
) -> ! {
	panic!("FATAL DOUBLE FAULT ERR:\n{:#?}", stack_frame)
}