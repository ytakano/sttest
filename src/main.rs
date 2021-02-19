use session_types as S;
use std::collections::HashMap;
use std::thread;

type Client = S::Send<u64, S::Choose<S::Recv<u64, S::Eps>, S::Recv<bool, S::Eps>>>;
type Server = <Client as S::HasDual>::Dual;

enum Op {
    Square,
    Even,
}

fn server(c: S::Chan<(), Server>) {
    let (c, n) = c.recv();
    match c.offer() {
        S::Branch::Left(c) => {
            c.send(n * n).close();
        }
        S::Branch::Right(c) => {
            c.send(n & 1 == 0).close();
        }
    }
}

fn client(c: S::Chan<(), Client>, n: u64, op: Op) {
    let c = c.send(n);
    match op {
        Op::Square => {
            let c = c.sel1();
            let (c, val) = c.recv();
            c.close();
            println!("{}^2 = {}", n, val);
        }
        Op::Even => {
            let c = c.sel2();
            let (c, val) = c.recv();
            c.close();
            if val {
                println!("{} is even", n);
            } else {
                println!("{} is odd", n);
            }
        }
    };
}

type Put = S::Recv<u64, S::Recv<u64, S::Var<S::Z>>>;
type Get = S::Recv<u64, S::Send<Option<u64>, S::Var<S::Z>>>;

type DBServer = S::Rec<S::Offer<Put, S::Offer<Get, S::Eps>>>;
type DBClient = <DBServer as S::HasDual>::Dual;

fn db_server(c: S::Chan<(), DBServer>) {
    let mut c_enter = c.enter();
    let mut db = HashMap::new();

    loop {
        match c_enter.offer() {
            S::Branch::Left(c) => {
                let (c, key) = c.recv();
                let (c, val) = c.recv();
                db.insert(key, val);
                c_enter = c.zero();
            }
            S::Branch::Right(c) => match c.offer() {
                S::Branch::Left(c) => {
                    let (c, key) = c.recv();
                    let c = if let Some(val) = db.get(&key) {
                        c.send(Some(*val))
                    } else {
                        c.send(None)
                    };
                    c_enter = c.zero();
                }
                S::Branch::Right(c) => {
                    c.close();
                    return;
                }
            },
        }
    }
}

fn db_client(c: S::Chan<(), DBClient>) {
    let c = c.enter();
    let c = c.sel1().send(10).send(4).zero();
    let c = c.sel1().send(50).send(7).zero();

    let (c, val) = c.sel2().sel1().send(10).recv();
    println!("val = {:?}", val);

    let c = c.zero();

    let (c, val) = c.sel2().sel1().send(20).recv();
    println!("val = {:?}", val);

    let _ = c.zero().sel2().sel2().close();
}

fn main() {
    let (server_chan, client_chan) = S::session_channel();
    let srv_t = thread::spawn(move || server(server_chan));
    let cli_t = thread::spawn(move || client(client_chan, 11, Op::Even));
    srv_t.join().unwrap();
    cli_t.join().unwrap();

    let (server_chan, client_chan) = S::session_channel();
    let srv_t = thread::spawn(move || server(server_chan));
    let cli_t = thread::spawn(move || client(client_chan, 11, Op::Square));
    srv_t.join().unwrap();
    cli_t.join().unwrap();

    let (server_chan, client_chan) = S::session_channel();
    let srv_t = thread::spawn(move || db_server(server_chan));
    let cli_t = thread::spawn(move || db_client(client_chan));
    srv_t.join().unwrap();
    cli_t.join().unwrap();
}
