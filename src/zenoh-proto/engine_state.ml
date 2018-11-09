open Apero
open Apero_net
open NetService
open R_name

module SIDMap = Map.Make(NetService.Id)

type tx_session_connector = Locator.t -> TxSession.t Lwt.t 

type engine_state = {
    pid : IOBuf.t;
    lease : Vle.t;
    locators : Locators.t;
    smap : Session.t SIDMap.t;
    rmap : Resource.t ResMap.t;      
    peers : Locator.t list;
    router : ZRouter.t;
    next_mapping : Vle.t;
    tx_connector : tx_session_connector;
    buffer_pool : IOBuf.t Lwt_pool.t

}

let report_resources e = 
    List.fold_left (fun s (_, r) -> s ^ Resource.report r ^ "\n") "" (ResMap.bindings e.rmap)