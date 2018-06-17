open Ztypes
open Frame
open Lwt
open Locator

module ZId = Id

module Transport : sig 

  module Id : ZId.S

  module  Info : sig 
      type kind = Packet| Stream  
      type t      
      val create : string -> Id.t -> bool -> kind -> int option -> t
      val name : t -> string
      val id : t -> Id.t
      val reliable : t -> bool
      val kind : t -> kind
      val mtu : t -> int option
      val compare : t -> t -> int
  end
  
  module Session : sig 
    
    module Id : ZId.S

    module Info : sig  
      type t
      val create : Id.t -> Locator.t -> Locator.t -> Info.t -> t
      val id : t -> Id.t
      val source : t -> Locator.t
      val dest : t -> Locator.t
      val transport_info : t -> Info.t
    end
  end

  module EventSource : sig 
    type event = 
      | SessionClose of Session.Id.t
      | SessionMessage of  Frame.t * Session.Id.t
      | LocatorMessage of Frame.t * Locator.t   
      | Events of event list

    type pull = unit -> event Lwt.t
  end
  
  module EventSink : sig 
    type event = 
      | SessionClose of Session.Id.t
      | SessionMessage of  Frame.t * Session.Id.t
      | LocatorMessage of Frame.t * Locator.t   
      | Events of event list      

    type push = event -> unit Lwt.t           
  end    
  
  
  module type S = sig       
    val info : Info.t
    val start : EventSink.push -> (EventSink.push * unit Lwt.t) Lwt.t
    val stop : unit -> unit Lwt.t
    val info : Info.t      
    val listen : Locator.t -> Session.Id.t Lwt.t
    val connect : Locator.t -> Session.Id.t Lwt.t
    val session_info : Session.Id.t -> Session.Info.t option
  end  

  module Engine : sig
  (** The [Transport.Engine] provides facilities for dyanmically loading transports 
      and abstracting them. *)
    type t        

    val create : unit -> t Lwt.t
    val add_transport : t -> (module S) -> Id.t Lwt.t
    val remove_transport : t -> Id.t -> bool Lwt.t
    val listen : t -> Locator.t -> Session.Id.t Lwt.t
    val connect : t -> Locator.t -> Session.Id.t Lwt.t
    val start : t -> EventSource.pull -> EventSink.push Lwt.t     
    val session_info : t -> Session.Id.t -> Session.Info.t option
  end
end
