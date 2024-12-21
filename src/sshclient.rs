use std::io::BufReader;
use std::io::prelude::*;
use std::net::{TcpStream};
use std::path::Path;
use colored::Colorize;
use ssh2::{OpenFlags, OpenType, Session};
use crate::error::SeeedError;

pub struct SshClient {
    session: Option<Session>,
    use_sudo: bool,
}

impl SshClient {

    pub fn new(use_sudo: bool) -> Self {
        Self {
            session: None,
            use_sudo,
        }
    }

    pub fn connect(&mut self, target: &str) -> Result<(), SeeedError> {

        // parse target
        let pattern = regex::Regex::new(r"^(?P<username>[^:@]+)@(?P<hostname>[^:]+)(:(?P<port>\d+))?$").unwrap();
        let captures = pattern.captures(target);
        let (username, host, port):(&str, &str, u16) = match captures {
            Some(captures) => {
                let host = captures.name("hostname").unwrap().as_str();   // unwrap because we know this wont fail
                let username = captures.name("username").unwrap().as_str();

                let port = match captures.name("port") {
                    Some(port) => port.as_str().parse::<u16>().unwrap(),
                    None => 22,
                };

                Ok((username, host, port))
            }
            None => {
                Err(SeeedError::BadTarget)
            }
        }?;

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

    pub fn command(&self, command: &str) -> Result<(), SeeedError> {
        let session = self.session.as_ref().unwrap().clone();
        let mut channel = session.channel_session()?;
        channel.exec(command)?;

        // read the output
        let mut stdout = String::new();
        channel.read_to_string(&mut stdout)?;

        channel.wait_close()?;
        Ok(())
    }

    pub fn run(&self, script: &str) -> Result<(), SeeedError> {

        let session = self.session.as_ref().unwrap().clone();

        let remote_script_path = format!("/var/lib/seeed/script_{}.sh", uuid::Uuid::new_v4());

        // upload the script to the remote target
        let sftp = session.sftp()?;
        let path = Path::new(remote_script_path.as_str());
        let mut file = sftp.create(path)?;
        file.write_all(script.as_bytes())?;
        file.close()?;

        // execute the script
        let mut channel = session.channel_session()?;
        channel.exec(format!("/bin/bash {}", remote_script_path).as_str())?;

        // pipe channel to a formater
        let mut stderr_reader = BufReader::new(channel.stderr());
        let mut stdout_reader = BufReader::new(channel);

        let mut line = String::new();
        loop {
            let r = stdout_reader.read_line(&mut line)?;
            if r == 0 {
                break;
            } else {
                print!("   | {}", line.yellow());  // print and not println, line already as the newline
                line.clear();
            }
        }
        loop {
            let r = stderr_reader.read_line(&mut line)?;
            if r == 0 {
                break;
            } else {
                print!("   | {}", line.red());  // print and not println, line already as the newline
                line.clear();
            }
        }

        // remove the script from the remote target
        sftp.unlink(path)?;

        Ok(())
    }

    pub(crate) fn upload(&self, content: &str, dst_path: String) -> Result<(), SeeedError> {
        let session = self.session.as_ref().unwrap().clone();

        let sftp = session.sftp()?;
        let path = Path::new(dst_path.as_str());
        let mut file = sftp.create(path)?;
        file.write_all(content.as_bytes())?;
        file.close()?;

        Ok(())
    }

}