open Zenoh

let locator = match Array.length Sys.argv with 
  | 1 -> "tcp/127.0.0.1:7447"
  | _ -> Sys.argv.(1)

let uri = match Array.length Sys.argv with 
  | 1 | 2 -> "/demo/example/zenoh-ocaml-stream"
  | _ -> Sys.argv.(2)

let value = match Array.length Sys.argv with 
  | 1 | 2 | 3 -> "Stream from OCaml!"
  | _ -> Sys.argv.(3)

let run =
  Printf.printf "Connecting to %s...\n%!" locator;
  let%lwt z = zopen locator in 

  Printf.printf "Declaring Publisher on '%s'...\n%!" uri;
  let%lwt pub = publish z uri in

  let idx = 0 in
  let rec loop = fun () ->
    let%lwt _ = Lwt_unix.sleep 1.0 in
    let s = Printf.sprintf "[%4d] %s" idx value in
    Printf.printf "Streaming Data ('%s': '%s')...\n%!" uri s;
    let%lwt _ = stream pub (Abuf.from_bytes @@ Bytes.of_string s) in
    loop ()
  in
  let%lwt _ = loop () in

  let%lwt _ = unpublish z pub in
  zclose z
  

let () = 
  Lwt_main.run @@ run
