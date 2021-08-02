use std::{
    io::Read,
    sync::mpsc::{self, Sender},
};
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    sync::mpsc::Receiver,
};

use tokio::spawn;
async fn read_message(mut cli: TcpStream, ser: Sender<Vec<u8>>) {
    loop {
        if cli.write(&[0]).is_err() {
            return;
        }
        let mut rec = Vec::new();
        let mut buf;
        loop {
            buf = vec![0; 511];
            if let Err(_) = cli.read(&mut buf) {
                return;
            }
            rec.append(&mut (buf.clone()));
            if buf[510] == 0 {
                break;
            }
        }
        if rec[0] != 0 {
            ser.send(rec).unwrap();
        }
    }
}
async fn receive_message(rec: Receiver<Vec<u8>>, clirec: Receiver<TcpStream>) {
    let mut clients = Vec::new();
    loop {
        let msg = rec.recv().unwrap();
        while let Ok(cli) = clirec.try_recv() {
            clients.push(cli);
        }
        clients = clients
            .into_iter()
            .filter_map(|mut client| client.write_all(&msg).map(|_| client).ok())
            .collect::<Vec<_>>();
    }
}
#[tokio::main]
async fn main() {
    let (sender, recever) = mpsc::channel();
    let (cli_sender, cli_recever) = mpsc::channel();
    println!("Input the address:");
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).unwrap();
    let listener = TcpListener::bind(s.trim()).unwrap();
    spawn(receive_message(recever, cli_recever));
    listener.incoming().for_each(|client| {
        let client = client.expect("link failed!");
        cli_sender
            .send(client.try_clone().expect("Failed when cloning client!"))
            .unwrap();
        spawn(read_message(client, sender.clone()));
    });
}
