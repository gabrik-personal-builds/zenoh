open Lwt.Infix
open Apero
open Apero_net
open NetService
open R_name
open Engine_state

open Discovery 

let pid_to_string = Abuf.hexdump

let make_scout = Message.Scout (Message.Scout.create Message.ScoutFlags.scoutBroker [])

let make_hello pe = Message.Hello (Message.Hello.create Message.ScoutFlags.scoutBroker pe.locators [])

let make_open pe = 
  Message.Open (Message.Open.create (char_of_int 0) pe.pid 0L pe.locators [ZProperty.NodeMask.make Message.ScoutFlags.scoutBroker])

let make_accept pe opid = Message.Accept (Message.Accept.create opid pe.pid pe.lease [])


let rec connect_peer peer connector max_retries = 
  let open Frame in 
  let open Apero.Infix in
  let open Session in
  let txconnector = (connector %> fun tslwt -> tslwt >>= fun ts -> Lwt.return (TxSex(ts))) in
  Lwt.catch 
    (fun () ->
       let%lwt _ = Logs_lwt.debug (fun m -> m "Connecting to peer %s" (Locator.to_string peer)) in 
       let%lwt tx_sex = txconnector peer in
       let frame = Frame.create [make_scout] in 
       let%lwt _ = Logs_lwt.debug (fun m -> m "Sending scout to peer %s" (Locator.to_string peer)) in 
       Mcodec.ztcp_write_frame_alloc tx_sex frame )
    (fun _ -> 
       let%lwt _ = Logs_lwt.debug (fun m -> m "Failed to connect to %s" (Locator.to_string peer)) in 
       let%lwt _ = Lwt_unix.sleep 1.0 in 
       if max_retries > 0 then connect_peer peer connector (max_retries -1)
       else Lwt.fail_with ("Permanently Failed to connect to " ^ (Locator.to_string peer)))

let connect_peers pe = 
  Lwt_list.iter_p (fun p -> 
      Lwt.catch
        (fun _ -> (connect_peer p pe.tx_connector 1000) >|= ignore )
        (fun ex -> let%lwt _ = Logs_lwt.warn (fun m -> m "%s" (Printexc.to_string ex)) in Lwt.return_unit)
    ) pe.peers


let remove_session pe tsex peer =    
  let sid = Session.txid tsex in 
  let%lwt _ = Logs_lwt.debug (fun m -> m  "Un-registering Session %s \n" (Id.to_string sid)) in
  let smap = SIDMap.remove sid pe.smap in
  let rmap = ResMap.map (fun r -> Resource.remove_mapping r sid) pe.rmap in 

  let optpeer = List.find_opt (fun (x:ZRouter.peer) -> Session.txid x.tsex = Session.txid tsex) pe.router.peers in
  let%lwt router = match optpeer with
    | Some peer ->
      let%lwt _ = Logs_lwt.debug (fun m -> m "Delete node %s" peer.pid) in
      let%lwt _ = Logs_lwt.debug (fun m -> m "Spanning trees status :\n%s" (ZRouter.report pe.router)) in
      Lwt.return @@ ZRouter.delete_node pe.router peer.pid
    | None -> Lwt.return pe.router in
  let pe = {pe with rmap; smap; router} in
  forward_all_decls pe;
  let%lwt pe = notify_all_pubs pe in
  Lwt.ignore_result @@ Lwt.catch
    (fun _ -> match Locator.of_string peer with 
       | Some loc -> if List.exists (fun l -> l = loc) pe.peers 
         then connect_peer loc pe.tx_connector 1000
         else Lwt.return 0
       | None -> Lwt.return 0)
    (fun ex -> let%lwt _ = Logs_lwt.warn (fun m -> m "%s" (Printexc.to_string ex)) in Lwt.return 0);
  Lwt.return pe


let guarded_remove_session engine tsex peer =
  let%lwt _ = Logs_lwt.debug (fun m -> m "Cleaning up session %s (%s) because of a connection drop" (Id.to_string  @@ Session.txid tsex) peer) in 
  Guard.guarded engine 
  @@ fun pe -> 
  let%lwt pe = remove_session pe tsex peer in
  Guard.return pe pe

let add_session engine tsex mask = 
  let open Session in
  Guard.guarded engine 
  @@ fun pe ->      
  let sid = Session.txid tsex in    
  let%lwt _ = Logs_lwt.debug (fun m -> m "Registering Session %s mask:%i\n" (Id.to_string sid) (Vle.to_int mask)) in
  let s = Session.create (tsex:Session.tx_sex) mask in    
  let smap = SIDMap.add (Session.txid tsex) s pe.smap in   
  let%lwt peer = 
    Lwt.catch 
      (fun () -> 
        match tsex with 
        | TxSex ts -> 
          (match (Lwt_unix.getpeername (TxSession.socket ts)) with 
          | Lwt_unix.ADDR_UNIX u -> Lwt.return u
          | Lwt_unix.ADDR_INET (a, p) -> Lwt.return @@ "tcp/" ^ (Unix.string_of_inet_addr a) ^ ":" ^ (string_of_int p))
        | Local _ -> Lwt.return "Local")
      (fun _ -> Lwt.return "UNKNOWN") in
  let _ = match tsex with 
  | TxSex ts -> Lwt.bind (TxSession.when_closed ts)  (fun _ -> guarded_remove_session engine tsex peer)
  | Local _ -> Lwt.return pe in
  let pe' = {pe with smap} in
  Guard.return pe' pe'


let process_scout engine _ _ = 
  Lwt.return [make_hello @@ Guard.get engine]

let process_hello engine tsex msg  =
  let sid = Session.txid tsex in 
  let%lwt pe' = add_session engine tsex (Message.Hello.mask msg) in           
  match Message.ScoutFlags.hasFlag (Message.Hello.mask msg) Message.ScoutFlags.scoutBroker with 
  | false -> Lwt.return  []
  | true -> (
      let%lwt _ = Logs_lwt.debug (fun m -> m "Try to open ZENOH session with broker on transport session: %s\n" (Id.to_string sid)) in
      Lwt.return [make_open pe'])

let process_broker_open engine tsex msg = 
  Guard.guarded engine 
  @@ fun pe ->
  let%lwt _ = Logs_lwt.debug (fun m -> m "Accepting Open from remote broker: %s\n" (pid_to_string @@ Message.Open.pid msg)) in
  let pe' = {pe with router = ZRouter.new_node pe.router {pid = Abuf.hexdump @@ Message.Open.pid msg; tsex}} in
  forward_all_decls pe;
  Guard.return [make_accept pe' (Message.Open.pid msg)] pe'

let process_open engine tsex msg  =
  let open Lwt.Infix in 
  let pe = Guard.get engine in
  match SIDMap.find_opt (Session.txid tsex) pe.smap with
  | None -> 
    (let mask = match ZProperty.NodeMask.find_opt (Message.Open.properties msg) with 
        | None -> 0L
        | Some nodeMask -> ZProperty.NodeMask.mask nodeMask in
     match Message.ScoutFlags.hasFlag mask (Message.ScoutFlags.scoutBroker) with 
     | false ->
       let%lwt _ = Logs_lwt.debug (fun m -> m "Accepting Open from unscouted remote peer: %s\n" (pid_to_string @@ Message.Open.pid msg)) in
       let%lwt pe' = add_session engine tsex mask in 
       Lwt.return [make_accept pe' (Message.Open.pid msg)] 
     | true -> 
       let%lwt pe' = add_session engine tsex mask in 
       Guard.set  pe' engine >>= 
       fun _ -> process_broker_open engine tsex msg)
  | Some session -> match Vle.logand session.mask Message.ScoutFlags.scoutBroker <> 0L with 
    | false -> 
      let%lwt _ = Logs_lwt.debug (fun m -> m "Accepting Open from remote peer: %s\n" (pid_to_string @@ Message.Open.pid msg)) in
      Lwt.return ([make_accept pe (Message.Open.pid msg)])     
    | true -> process_broker_open engine tsex msg

let process_accept_broker engine tsex msg = 
  Guard.guarded engine
  @@ fun pe ->
  let%lwt _ = Logs_lwt.debug (fun m -> m "Accepted from remote broker: %s\n" (pid_to_string @@ Message.Accept.apid msg)) in
  let pe' = {pe with router = ZRouter.new_node pe.router {pid = Abuf.hexdump @@ Message.Accept.apid msg; tsex}} in
  forward_all_decls pe;
  Guard.return [] pe'

let process_accept engine tsex msg =
  let pe = Guard.get engine in
  let sid = Session.txid tsex in 
  match SIDMap.find_opt sid pe.smap with
  | None -> 
    let%lwt _ = Logs_lwt.debug (fun m -> m "Accepted from unscouted remote peer: %s\n" (pid_to_string @@ Message.Accept.apid msg)) in
    let%lwt _ = add_session engine tsex Vle.zero in  Lwt.return [] 
  | Some session -> match Message.ScoutFlags.hasFlag session.mask Message.ScoutFlags.scoutBroker with 
    | false -> (
        let%lwt _ = Logs_lwt.debug (fun m -> m "Accepted from remote peer: %s\n" (pid_to_string @@ Message.Accept.apid msg)) in
        Lwt.return [])      
    | true -> process_accept_broker engine tsex msg 

let process_close (engine) _ = 
  let pe = Guard.get engine in
  Lwt.return [Message.Close (Message.Close.create pe.pid '0')]
