use libc::{openpty, fork, close, execvp, _exit, c_char, c_int};
use std::ffi::CString;
use std::ptr;

pub struct PTY {
    pub pid: libc::pid_t,
    pub master: i32,
    pub slave: i32,
}

impl PTY {
    pub fn new(command: &str, args: &[&str]) -> Result<Self, String> {
        let mut master: c_int = -1;
        let mut slave: c_int = -1;
        let mut name: [c_char; 1024] = [0; 1024];
        
        unsafe {
            let result = openpty(
                &mut master,
                &mut slave,
                name.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
            );
            
            if result < 0 {
                return Err(format!("openpty failed: {}", std::io::Error::last_os_error()));
            }
        }
        
        let pid = unsafe { fork() };
        
        if pid < 0 {
            unsafe {
                close(master);
                close(slave);
            }
            return Err(format!("fork failed: {}", std::io::Error::last_os_error()));
        }
        
        if pid == 0 {
            unsafe {
                close(master);
                
                let sid = libc::setsid();
                if sid < 0 {
                    _exit(1);
                }
                
                libc::ioctl(slave, libc::TIOCSCTTY, 0 as *mut libc::c_void);
                
                let dup2_result = libc::dup2(slave, 0);
                if dup2_result < 0 {
                    _exit(1);
                }
                let dup2_result = libc::dup2(slave, 1);
                if dup2_result < 0 {
                    _exit(1);
                }
                let dup2_result = libc::dup2(slave, 2);
                if dup2_result < 0 {
                    _exit(1);
                }
                
                close(slave);
                
                let cmd_cstring = CString::new(command).unwrap();
                let arg_cstrings: Vec<CString> = args.iter()
                    .map(|s| CString::new(*s).unwrap())
                    .collect();
                let mut argv: Vec<*const c_char> = arg_cstrings.iter()
                    .map(|s| s.as_ptr())
                    .collect();
                argv.push(ptr::null());
                
                execvp(cmd_cstring.as_ptr(), argv.as_ptr());
                _exit(1);
            }
        }
        
        unsafe {
            close(slave);
        }
        
        Ok(PTY { pid, master, slave })
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, String> {
        unsafe {
            let written = libc::write(self.master, data.as_ptr() as *const libc::c_void, data.len());
            if written < 0 {
                Err(format!("Write failed: {}", std::io::Error::last_os_error()))
            } else {
                Ok(written as usize)
            }
        }
    }

    pub fn read(&self) -> Result<Vec<u8>, String> {
        let mut buf = [0u8; 8192];
        unsafe {
            let read = libc::read(self.master, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if read < 0 {
                Err(format!("Read failed: {}", std::io::Error::last_os_error()))
            } else {
                Ok(buf[..read as usize].to_vec())
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        unsafe {
            let mut status: libc::c_int = 0;
            let result = libc::waitpid(self.pid, &mut status, libc::WNOHANG);
            result == 0
        }
    }

    pub fn close(&mut self) {
        unsafe {
            if self.master >= 0 {
                libc::close(self.master);
            }
            if self.slave >= 0 {
                libc::close(self.slave);
            }
        }
    }
}

impl Drop for PTY {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.pid, libc::SIGTERM);
            let mut status: libc::c_int = 0;
            libc::waitpid(self.pid, &mut status, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_creation() {
        let pty = PTY::new("ls", &["-la"]).unwrap();
        assert!(pty.master >= 0);
    }

    #[test]
    fn test_write_to_pty() {
        let mut pty = PTY::new("cat", &[]).unwrap();
        pty.write(b"hello\n").unwrap();
    }

    #[test]
    fn test_read_from_pty() {
        let pty = PTY::new("echo", &["hello"]).unwrap();
        let output = pty.read().unwrap();
        assert!(output.len() > 0);
    }
}
