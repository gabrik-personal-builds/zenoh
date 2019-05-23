open Cmdliner

let zopen argv = 
  let run tcpport peers strength bufn timestamp = 
    let (instream, inpush) = Lwt_stream.create_bounded 256 in
    let (outstream, outpush) = Lwt_stream.create_bounded 256 in
    Zengine.run tcpport peers strength bufn timestamp (Some (instream, outpush)) |> Lwt.ignore_result; 
    Zenoh.zropen (outstream, inpush)
  in 
  
  Term.(eval (const run $ Zengine.tcpport $ Zengine.peers $ Zengine.strength $ Zengine.bufn $ Zengine.timestamp, Term.info "zenohd") ~argv) |> function
  | `Ok result -> result
  | _ -> Lwt.fail_with "Invalid arguments"
  