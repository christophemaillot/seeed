
use std::io::prelude::*;
use std::net::{TcpStream};
use std::path::Path;
use std::thread;
use colored::Colorize;
use ssh2::Session;
use crate::error::SeeedError;

pub trait RemoteExecutor {
    fn connect(&mut self, target: &str) -> Result<(), SeeedError>;
    fn command(&self, command: &str) -> Result<(), SeeedError>;
    fn run(&self, script: &str) -> Result<(), SeeedError>;
    fn upload(&self, content: &str, dst_path: String) -> Result<(), SeeedError>;
}

pub struct SshClient {
    session: Option<Session>,
    use_sudo: bool,
}

impl RemoteExecutor for SshClient {
    fn connect(&mut self, target: &str) -> Result<(), SeeedError> {
        self.connect_impl(target)
    }

    fn command(&self, command: &str) -> Result<(), SeeedError> {
        self.command_impl(command)
    }

    fn run(&self, script: &str) -> Result<(), SeeedError> {
        self.run_impl(script)
    }

    fn upload(&self, content: &str, dst_path: String) -> Result<(), SeeedError> {
        self.upload_impl(content, dst_path)
    }
}

impl SshClient {

    pub fn new(use_sudo: bool) -> Self {
        Self {
            session: None,
            use_sudo,
        }
    }

    fn connect_impl(&mut self, target: &str) -> Result<(), SeeedError> {

        // parse target
        let pattern = regex::Regex::new(r"^(?P<username>[^:@]+)@(?P<hostname>[^:]+)(:(?P<port>\d+))?$")?;
        let captures = pattern.captures(target).ok_or(SeeedError::BadTarget)?;

        let host = captures.name("hostname").ok_or(SeeedError::BadTarget)?.as_str();
        let username = captures.name("username").ok_or(SeeedError::BadTarget)?.as_str();

        let port = match captures.name("port") {
            Some(port) => port.as_str().parse::<u16>().map_err(|_| SeeedError::BadTarget)?,
            None => 22,
        };

        let (username, host, port) = (username, host, port);

        // register the target
        let target = format!("{}:{}",  host, port);

        // issue the connect process
        let tcp = TcpStream::connect(target)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // try to authenticate using the ssh agent
        let mut agent = session.agent()?;
        agent.connect()?;
        agent.list_identities()?;
        let identities = agent.identities()?;

        let mut authenticated = false;

        for identity in identities.iter() {
            match agent.userauth(username, identity) {
                Ok(_) => {
                    authenticated = true;
                    break

                },
                Err(_) => continue,
            }
        }

        if authenticated == false {
            return Err(SeeedError::BadTarget)
        }

        self.session = Some(session);

        Ok(())
    }

    fn command_impl(&self, command: &str) -> Result<(), SeeedError> {
        let session = self.session.as_ref().ok_or(SeeedError::GenericSshError("Session not initialized".to_string()))?.clone();
        let mut channel = session.channel_session()?;
        channel.exec(command)?;

        // read the output
        let mut stdout = String::new();
        channel.read_to_string(&mut stdout)?;

        channel.wait_close()?;
        Ok(())
    }

    fn run_impl(&self, script: &str) -> Result<(), SeeedError> {

        let session = self.session.as_ref().ok_or(SeeedError::GenericSshError("Session not initialized".to_string()))?.clone();

        let remote_script_path = format!("/var/lib/seeed/script_{}.sh", uuid::Uuid::new_v4());

        // upload the script to the remote target
        let sftp = session.sftp()?;
        let path = Path::new(remote_script_path.as_str());
        let mut file = sftp.create(path)?;
        file.write_all(script.as_bytes())?;
        file.close()?;

        // execute the script
        let mut channel = session.channel_session()?;
        if self.use_sudo {
            channel.exec(format!("sudo /bin/bash {}", remote_script_path).as_str())?;
        } else {
            channel.exec(format!("/bin/bash {}", remote_script_path).as_str())?;
        }


        // pipe channel to a formater

        // pipe channel to a formater
        // Set non-blocking to true to enable polling
        session.set_blocking(false);

        let mut stdout_buf: Vec<u8> = Vec::new();
        let mut stderr_buf: Vec<u8> = Vec::new();
        let mut buff = [0u8; 1024];

        let mut stdout_done = false;
        let mut stderr_done = false;

        loop {
            let mut made_progress = false;

            // Read stdout
            if !stdout_done {
                match channel.read(&mut buff) {
                    Ok(0) => { stdout_done = true; }
                    Ok(n) => {
                        made_progress = true;
                        stdout_buf.extend_from_slice(&buff[..n]);
                        while let Some(pos) = stdout_buf.iter().position(|&b| b == b'\n') {
                            let line_bytes = stdout_buf.drain(..=pos).collect::<Vec<u8>>();
                            let line = String::from_utf8_lossy(&line_bytes);
                            // Print with newline as originally intended (line includes \n)
                            print!("   | {}", line.yellow());
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(SeeedError::GenericSshError(e.to_string())),
                }
            }

            // Read stderr
            if !stderr_done {
                match channel.stderr().read(&mut buff) {
                    Ok(0) => { stderr_done = true; }
                    Ok(n) => {
                        made_progress = true;
                        stderr_buf.extend_from_slice(&buff[..n]);
                        while let Some(pos) = stderr_buf.iter().position(|&b| b == b'\n') {
                            let line_bytes = stderr_buf.drain(..=pos).collect::<Vec<u8>>();
                            let line = String::from_utf8_lossy(&line_bytes);
                            print!("   | {}", line.red());
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(SeeedError::GenericSshError(e.to_string())),
                }
            }

            if stdout_done && stderr_done {
                break;
            }

            if !made_progress {
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        // Print any remaining content in buffers
        if !stdout_buf.is_empty() {
            let line = String::from_utf8_lossy(&stdout_buf);
            print!("   | {}", line.yellow());
            if !line.ends_with('\n') { println!(); }
        }
        if !stderr_buf.is_empty() {
             let line = String::from_utf8_lossy(&stderr_buf);
             print!("   | {}", line.red());
             if !line.ends_with('\n') { println!(); }
        }

        session.set_blocking(true);

        // wait for the script to finish
        // @todo

        // remove the script from the remote target
        sftp.unlink(path)?;

        Ok(())
    }

    fn upload_impl(&self, content: &str, dst_path: String) -> Result<(), SeeedError> {
        
        let effective_dst_path: String = if self.use_sudo {
            format!("/var/lib/seeed/upload_{}.data", uuid::Uuid::new_v4())
        } else {
            dst_path.clone()
        };

        let session = self.session.as_ref().ok_or(SeeedError::GenericSshError("Session not initialized".to_string()))?.clone();

        let sftp = session.sftp()?;
        let path = Path::new(effective_dst_path.as_str());
        let mut file = sftp.create(path)?;
        file.write_all(content.as_bytes())?;
        file.close()?;

        if self.use_sudo {
            let mut channel = session.channel_session()?;
            channel.exec(format!("sudo mv {} {}", effective_dst_path, dst_path).as_str())?;
        }

        Ok(())
    }

}