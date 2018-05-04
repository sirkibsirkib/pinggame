// poll.register(&sock, client_tok, Ready::readable(),
//               PollOpt::edge()).unwrap();
// let mut events = Events::with_capacity(128);
// loop {
//     poll.poll(&mut events, None).unwrap();
//     for event in events.iter() {
//         match event.token() {
//             client_tok => {
//         		println!("client connects");
//                 // The server just shuts down the socket, let's just exit
//                 // from our event loop.
//                 return Ok(());
//             }
//             _ => unreachable!(),
//         }
//     }
// }