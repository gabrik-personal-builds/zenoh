open Ztypes
open Zlocator
open Ziobuf
open Zmessage
open Zmessage.Message
open Lwt
open Zlwt
open Zframe

module Marshaller = struct
let read_seq read buf  =
  let rec read_remaining buf seq length =
    match length with
    | 0 -> return (seq, buf)
    | _ ->
      let%lwt (value, buf) = read buf in
      read_remaining buf (value :: seq) (length - 1)
  in
  let%lwt (length, buf) = IOBuf.get_vle buf in
  let%lwt _ = Lwt_log.debug @@  (Printf.sprintf "Reading seq of %d elements" (Vle.to_int length)) in
  read_remaining buf [] (Vle.to_int length)

let write_seq write buf seq  =
  let rec write_remaining buf seq =
    match seq with
    | [] -> return buf
    | head :: rem ->
      let%lwt buf = write buf head in
      write_remaining buf rem
  in
  let%lwt buf = IOBuf.put_vle buf (Vle.of_int (List.length seq)) in
  write_remaining buf seq

let read_prop buf =
  let%lwt (id, buf) = IOBuf.get_vle buf in
  let%lwt (data, buf) = IOBuf.get_io_buf buf in
  return (Property.create id data, buf)

let write_prop buf prop =
  let (id, value) = prop in
  let%lwt buf = IOBuf.put_vle buf id in
  IOBuf.put_io_buf buf value

let read_prop_seq = read_seq read_prop

let write_prop_seq  = write_seq write_prop

let write_properties buf ps =
  if ps = Properties.empty then return buf
  else write_prop_seq buf ps


let read_properties buf h =
  if Flags.(hasFlag h pFlag) then read_prop_seq buf
  else return (Properties.empty, buf)

let read_locator buf =
  let%lwt (str, buf) = IOBuf.get_string buf in
  return (Locator.of_string str, buf)

let write_locator buf locator =
  IOBuf.put_string buf (Locator.to_string locator)

let read_locator_seq = read_seq read_locator

let write_locator_seq = write_seq write_locator

let read_scout buf header =
  let%lwt _ =  Lwt_log.debug "Rading Scout" in
  let%lwt (mask, buf) = IOBuf.get_vle buf in
  let%lwt (ps, buf) = read_properties buf header in
  return (Scout (Scout.create mask ps), buf)


let write_scout buf scout =
  let open Scout in
  let%lwt _ = Lwt_log.debug "Writring Scout" in
  let%lwt buf = IOBuf.put_char buf (header scout) in
  let%lwt buf = IOBuf.put_vle buf (mask scout) in
  let%lwt buf =  write_properties buf (properties scout) in
  return buf

let read_hello buf header =
  let%lwt _ = Lwt_log.debug "Readings Hello" in
  let%lwt (mask, buf) = IOBuf.get_vle buf in
  let%lwt (locators, buf) = read_locator_seq buf in
  let%lwt (ps, buf) = read_properties buf header in
  return (Hello (Hello.create mask locators ps), buf)

let write_hello buf hello =
  let open Hello in
  let%lwt _ = Lwt_log.debug "Writing Hello" in
  let%lwt buf = IOBuf.put_char buf (header hello) in
  let%lwt buf = IOBuf.put_vle buf (mask hello) in
  let%lwt buf = write_locator_seq buf (locators hello) in
  write_properties buf (properties hello)

let read_open buf header =
  let%lwt _ =  Lwt_log.debug "Reading Open" in
  let%lwt (version, buf) = IOBuf.get_char buf in
  let%lwt (pid, buf) = IOBuf.get_io_buf buf in
  let%lwt (lease, buf) = IOBuf.get_vle buf in
  let%lwt(locs, buf) = read_locator_seq buf in
  let%lwt (ps, buf) = read_properties buf header in
  return (Open (Open.create version pid lease locs ps), buf)

let write_open buf msg =
  let open Open in
  let%lwt _ = Lwt_log.debug "Writing Open" in
  let%lwt buf = IOBuf.put_char buf (header msg) in
  let%lwt buf = IOBuf.put_char buf (version msg) in
  let%lwt buf = IOBuf.put_io_buf buf (pid msg) in
  let%lwt buf = IOBuf.put_vle buf (lease msg) in
  let%lwt buf = write_locator_seq buf (locators msg) in
  write_properties buf (properties msg)

let read_accept buf header =
  let%lwt _ = Lwt_log.debug "Reading Accept" in
  let%lwt (opid, buf) = IOBuf.get_io_buf buf in
  let%lwt (apid, buf) = IOBuf.get_io_buf buf in
  let%lwt (lease, buf) = IOBuf.get_vle buf in
  let%lwt (ps, buf) = read_properties buf header in
  return (Accept (Accept.create opid apid lease ps), buf)


let write_accept buf accept =
  let open Accept in
  let%lwt _ = Lwt_log.debug "Writing Accept" in
  let%lwt buf = IOBuf.put_char buf (header accept) in
  let%lwt buf = IOBuf.put_io_buf buf (opid accept) in
  let%lwt buf = IOBuf.put_io_buf buf (apid accept) in
  let%lwt buf = IOBuf.put_vle buf (lease accept) in
  let%lwt buf = write_properties buf (properties accept) in
  return buf


let read_close buf header =
  let%lwt _ = Lwt_log.debug "Reading Close" in
  let%lwt (pid, buf) = IOBuf.get_io_buf buf in
  let%lwt (reason, buf) = IOBuf.get_char buf in
  return (Close (Close.create pid reason), buf)

let write_close buf close =
  let open Close in
  let _ = Lwt_log.debug "Writing Close" in
  let%lwt buf = IOBuf.put_char buf (header close) in
  let%lwt buf = IOBuf.put_io_buf buf (pid close) in
  let%lwt buf = IOBuf.put_char buf (reason close) in
  return buf

let read_pub_decl buf h =
  let open PublisherDecl in
  let%lwt _ = Lwt_log.debug "Reading PubDeclaration" in
  let%lwt (rid, buf) = IOBuf.get_vle buf in
  let%lwt _ = Lwt_log.debug (Printf.sprintf "Reading PubDeclaration for rid = %Ld" rid) in
  let%lwt (ps, buf) = read_properties buf h in
  return (Declaration.PublisherDecl (PublisherDecl.create rid ps), buf)

let write_pub_decl buf d =
  let open PublisherDecl in
  let%lwt _ = Lwt_log.debug "Writing PubDeclaration" in
  let%lwt buf = IOBuf.put_char buf (header d) in
  let id = (rid d) in
  let%lwt _ = Lwt_log.debug (Printf.sprintf "Writing PubDeclaration for rid = %Ld" id) in
  let%lwt buf = IOBuf.put_vle buf id in
  write_properties buf (properties d)

let read_temporal_properties buf =
  let%lwt _ =  Lwt_log.debug "Reading TemporalProperties" in
  let%lwt (origin, buf) = IOBuf.get_vle buf in
  let%lwt (period, buf) = IOBuf.get_vle buf in
  let%lwt (duration, buf) = IOBuf.get_vle buf in
  return (TemporalProperties.create origin period duration, buf)

let write_temporal_properties buf stp =
  let open TemporalProperties in
  let open IOBuf in
  match stp with
  | None -> return buf
  | Some tp ->
    let%lwt _ = Lwt_log.debug "Writing Temporal" in
    let%lwt buf = put_vle buf (origin tp) in
    let%lwt buf = put_vle buf (period tp) in
    put_vle buf (duration tp)

let read_sub_mode buf =
  let%lwt _ = Lwt_log.debug "Reading SubMode" in
  match%lwt IOBuf.get_char buf with
  | (id, buf) when  Flags.mid id = SubscriptionModeId.pushModeId ->
    return (SubscriptionMode.PushMode, buf)

  | (id, buf) when  Flags.mid id =  SubscriptionModeId.pullModeId ->
    return (SubscriptionMode.PullMode, buf)

  | (id, buf) when  Flags.mid id =  SubscriptionModeId.periodicPushModeId ->
    let%lwt (tp, buf) = read_temporal_properties buf in
    return (SubscriptionMode.PeriodicPushMode tp, buf)

  | (id, buf) when  Flags.mid id =  SubscriptionModeId.periodicPullModeId ->
    let%lwt (tp, buf) = read_temporal_properties buf in
    return (SubscriptionMode.PeriodicPullMode tp, buf)

  | _ -> fail (ZError Error.(OutOfBounds NoMsg))


let write_sub_mode buf m =
  let open SubscriptionMode in
  let%lwt _ = Lwt_log.debug "Writing SubMode" in
  let sid = id m in
  let%lwt buf = IOBuf.put_char buf sid in
  write_temporal_properties buf (temporal_properties m)

let read_sub_decl buf h =
  let open SubscriberDecl in
  let%lwt _ = Lwt_log.debug "Reading SubDeclaration" in
  let%lwt (rid, buf) = IOBuf.get_vle buf in
  let%lwt (mode, buf) = read_sub_mode buf in
  let%lwt (ps, buf) = read_properties buf h in
  return (Declaration.SubscriberDecl (SubscriberDecl.create rid mode ps), buf)

let write_sub_decl buf d =
  let open SubscriberDecl in
  let%lwt _ =  Lwt_log.debug "Writing SubDeclaration" in
  let%lwt buf = IOBuf.put_char buf (header d) in
  let id = (rid d) in
  let%lwt _ = Lwt_log.debug (Printf.sprintf "Writing SubDeclaration for rid = %Ld" id) in
  let%lwt buf = IOBuf.put_vle buf id in
  let%lwt buf = write_sub_mode buf (mode d) in
  write_properties buf (properties d)

let read_commit_decl buf _ =
  let open IOBuf in
  let%lwt _ =  Lwt_log.debug "Reading Commit Decl" in
  let%lwt (commit_id, buf) = get_char buf in
  return ((Declaration.CommitDecl (CommitDecl.create commit_id)), buf)

let write_commit_decl buf cd =
  let open CommitDecl in
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Writing CommitDecl" in
  let%lwt buf = put_char buf (header cd) in
  put_char buf (commit_id cd)

let read_result_decl buf _ =
  let open IOBuf in
  let open Infix in
  let%lwt (commit_id, buf) = get_char buf in
  match%lwt get_char buf with
  | (status, buf) when status = char_of_int 0 ->
      return (Declaration.ResultDecl  (ResultDecl.create commit_id status None), buf)
  | (status, buf) ->
    let%lwt (v, buf) = get_vle buf in
    return (Declaration.ResultDecl (ResultDecl.create commit_id status (Some v)), buf)

let write_result_decl buf rd =
  let open ResultDecl in
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Writing Result Declaration" in
  let%lwt buf = put_char buf (header rd) in
  let%lwt buf = put_char buf (commit_id rd) in
  let%lwt buf = put_char buf (status rd) in
  match (id rd) with | None -> return buf | Some v -> put_vle buf v


let read_declaration buf =
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Reading Declaration" in
  let%lwt (header, buf) = get_char buf in
  let%lwt _ =  Lwt_log.debug @@ Printf.sprintf "Declaration Id = %d" (Header.mid header) in
  match Flags.mid header with
  | p when p = DeclarationId.publisherDeclId -> read_pub_decl buf header
  | s when s = DeclarationId.subscriberDeclId -> read_sub_decl buf header
  | c when c = DeclarationId.commitDeclId ->
    read_commit_decl buf header
  | r when r = DeclarationId.resultDeclId -> read_result_decl buf header
  | _ -> fail @@ ZError Error.NotImplemented


let write_declaration buf (d: Declaration.t) =
  match d with
  | PublisherDecl pd -> write_pub_decl buf pd
  | SubscriberDecl sd -> write_sub_decl buf sd
  | CommitDecl cd -> write_commit_decl buf cd
  | ResultDecl rd -> write_result_decl buf rd
  | _ -> fail @@ ZError Error.NotImplemented


let read_declarations buf =
  let open IOBuf in

  let rec loop buf n ds =
    if n = 0 then return (ds, buf)
    else
        let%lwt (d, buf) = read_declaration buf in
        loop buf (n-1) (d::ds)
  in
  let%lwt _ = Lwt_log.debug "Reading Declarations" in
  let%lwt (len, buf) = get_vle buf in
  let _ = Lwt_log.debug @@ Printf.sprintf "Parsing %Ld declarations" len in
  loop buf (Vle.to_int len) []

let write_declarations buf ds =
  let open Declare in
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Writing Declarations" in
  let%lwt buf = put_vle buf  @@ Vle.of_int @@ List.length @@ ds in
  LwtM.fold_m (fun b d -> write_declaration b d) buf ds

let read_declare buf h =
  let open IOBuf in

  let%lwt _ =  Lwt_log.debug "Reading Declare message" in
  let%lwt (sn, buf) = get_vle buf in
  let%lwt (ds, buf) = read_declarations buf in
  return (Declare (Declare.create ((Flags.hasFlag h Flags.sFlag), (Flags.hasFlag h Flags.cFlag)) sn ds), buf)

let write_declare buf decl =
  let open Declare in
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Writing Declare message" in
  let%lwt buf = put_char buf (header decl) in
  let%lwt buf = put_vle buf (sn decl) in
  let%lwt buf = write_declarations buf (declarations decl) in
  return buf

let read_stream_data buf h =
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Reading StreamData" in
  let%lwt (sn, buf) = get_vle buf in
  let%lwt (id, buf) = get_vle buf in
  let%lwt (prid, buf) =
    if Flags.(hasFlag h aFlag) then
      let%lwt (v, buf) = get_vle buf in return (Some v, buf)
    else return (None, buf)
  in
  let%lwt (payload, buf) = get_io_buf buf in
  let (r, s) = ((Flags.hasFlag h Flags.sFlag), (Flags.hasFlag h Flags.rFlag)) in
  return  (StreamData (StreamData.create (r, s) sn id prid payload),buf)

let write_stream_data buf m =
  let open StreamData in
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Writing StreamData" in
  let%lwt buf = put_char buf @@ header m in
  let%lwt buf = put_vle buf @@ sn m in
  let%lwt buf = put_vle buf @@ id m in
  let%lwt buf =
    match prid m with
    | None -> return buf
    | Some v -> put_vle buf v
  in put_io_buf buf @@ payload m

let read_synch buf h =
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Reading Synch" in
  let%lwt (sn, buf) = get_vle buf in
  let  (s, r) = Flags.(hasFlag h sFlag, hasFlag h rFlag) in
  if Flags.(hasFlag h uFlag) then
    let%lwt (c, buf) = get_vle buf in return (Synch (Synch.create (s,r) sn (Some c)), buf)
    else
      return (Synch (Synch.create (s,r) sn None), buf)

let write_synch buf m =
  let open Synch in
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Writing Synch" in
  let%lwt buf = put_char buf @@ header m in
  let%lwt buf = put_vle buf @@ sn m in
  match count m  with
  | None -> return buf
  | Some c -> put_vle buf c


let read_ack_nack buf h =
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Reading AckNack" in
  let%lwt (sn, buf) = get_vle buf in
  if Flags.(hasFlag h mFlag) then
    let%lwt (m, buf) = get_vle buf in
    return (AckNack (AckNack.create sn (Some m)), buf)
    else
      return (AckNack (AckNack.create sn None), buf)

let write_ack_nack buf m =
  let open AckNack in
  let open IOBuf in
  let%lwt _ = Lwt_log.debug "Writing AckNack" in
  let%lwt buf = put_char buf @@ header m in
  let%lwt buf = put_vle buf @@ sn m in
  match mask m with
  | None -> return buf
  | Some v -> put_vle buf v

let read_keep_alive buf header =
  let%lwt _ = Lwt_log.debug "Reading KeepAlive" in
  let%lwt (pid, buf) =  IOBuf.get_io_buf buf in
  return (KeepAlive (KeepAlive.create pid), buf)

let write_keep_alive buf keep_alive =
  let open KeepAlive in
  let open IOBuf in
  let%lwt _ =  Lwt_log.debug "Writing KeepAlive" in
  let%lwt buf =  put_char buf (header keep_alive) in
  put_io_buf buf (pid keep_alive)

let read_msg buf =
  let%lwt (header, buf) = IOBuf.get_char buf in
  let%lwt _ = Lwt_log.debug (Printf.sprintf "Received message with id: %d\n" (Header.mid header)) in
  match char_of_int (Header.mid (header)) with
  | id when id = MessageId.scoutId ->  (read_scout buf header)
  | id when id = MessageId.helloId ->  (read_hello buf header)
  | id when id = MessageId.openId ->  (read_open buf header)
  | id when id = MessageId.acceptId -> (read_accept buf header)
  | id when id = MessageId.closeId ->  (read_close buf header)
  | id when id = MessageId.declareId -> (read_declare buf header)
  | id when id = MessageId.sdataId ->  (read_stream_data buf header)
  | id when id = MessageId.synchId -> (read_synch buf header)
  | id when id = MessageId.ackNackId -> (read_ack_nack buf header)
  | id when id = MessageId.keepAliveId -> (read_keep_alive buf header)
  | uid ->
    let%lwt _ = Lwt_log.debug @@ Printf.sprintf "Received unknown message id: %d" (int_of_char uid) in
    fail @@ ZError Error.(InvalidFormat NoMsg)


let write_msg buf msg =
  match msg with
  | Scout m -> write_scout buf m
  | Hello m -> write_hello buf m
  | Open m -> write_open buf m
  | Accept m -> write_accept buf m
  | Close m -> write_close buf m
  | Declare m -> write_declare buf m
  | StreamData m -> write_stream_data buf m
  | Synch m -> write_synch buf m
  | AckNack m -> write_ack_nack buf m
  | KeepAlive m -> write_keep_alive buf m

  let read_frame_length buf = IOBuf.get_vle buf

  let write_frame_length buf f = IOBuf.put_vle buf (Vle.of_int @@ Frame.length f)

  let read_frame buf =
    let rec rloop buf n msgs =
      if n = 0 then return (msgs, buf)
      else
        let%lwt (msg, buf) = read_msg buf in
        rloop buf (n-1) (msg::msgs)
    in
    let%lwt (len, buf) = read_frame_length buf in
    let%lwt (msgs, buf) = rloop buf (Vle.to_int len) [] in
    return @@ (Frame.create msgs, buf)

  let write_frame buf f =
    let open Zlwt in
    LwtM.fold_m write_msg buf (Frame.to_list f)
end