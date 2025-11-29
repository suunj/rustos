
use volatile::Volatile;         // 컴파일러 최적화 방지
use core::fmt;                  // formatting
use lazy_static::lazy_static;   // runtime에 초기화되는 static 변수
use spin::Mutex;                // os 없이 사용가능한 spinlock mutex

// global writer instance
lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

// vga 색상 정의
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)] // 각 값이 u8로 저장됨을 보장
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

// vga buffer 의 단일 문자 셀
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

// vga text buffer
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// vga buffer에 문자를 쓰는 Writer
pub struct Writer {
    column_position: usize,         // current cusor 의 열위치
    color_code: ColorCode,          // 현재 사용중인 색상
    buffer: &'static mut Buffer,    // VGA buffer에 대한 참조
}

impl Writer {
    // 단일 바이트를 화면에 출력
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(), // 개행 문자면 새줄로
            byte => {
                // 줄 끝에 도달하면 새줄로
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1; // 항상 마지막행에 출력
                let col = self.column_position;
                let color_code = self.color_code;

                // vga buffer에 문자 쓰기
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    // 문자열을 화면에 출력
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // 출력가능한 ASCII 범위와 개행
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // 범위 밖의 문자 처리
                _ => self.write_byte(0xfe),
            }
        }
    }

    // 새줄로 이동
    // 모든 행을 한줄씩 위로 복사하고 마지막행을 지움
    fn new_line(&mut self) {
        // 모든 행을 한줄씩 위로 복사
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        // 마지막 행 지우기
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    // 특정 행을 공백으로 채움
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

// core::fmt::Write 트레이트 구현
// write! 및 writeln! 매크로 사용가능
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// print! 매크로
// $crate: 이 매크로가 정의된 크레이트를 참조(다른 모듈에서도 작동하도록)
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

// println! 매크로
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)] // 문서에서 숨김(공개 API가 아님)
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap(); // mutex 잠금 후 출력
}
