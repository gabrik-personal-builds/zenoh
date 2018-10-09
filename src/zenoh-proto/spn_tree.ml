open Sexplib.Std
open Printf


module Node = struct
  type id = string [@@deriving sexp]

  type t = {
    node_id  : id;
    tree_nb  : int;
    priority : (int * id);
    distance : int;
    parent   : id option;
    rank     : int
  }  [@@deriving sexp]

  let compare t1 t2 =
    let c1 = compare t1.rank t2.rank in
    if c1 <> 0 then c1 else 
    begin
      let c2 = compare t1.priority t2.priority in
      if c2 <> 0 then c2 else compare t2.distance t1.distance
    end
end

type t = {
  local : Node.t;
  peers : Node.t list;
}
type tree=t

module type S = sig
  val update : t -> Node.t -> t
  val delete_node : t -> Node.id -> t
  val is_stable : t -> bool
  val get_parent : t -> Node.t option
  val get_childs : t -> Node.t list
  val get_broken_links : t -> Node.t list
  val print : t -> unit
end

module type Configuration = sig
  val local_id : Node.id
  val local_prio : int
  val max_dist : int
  val max_trees : int
end

module Configure(Conf : Configuration) : S = struct
  open Node

  let update tree node =
    let open Node in
    {
      local =
        if compare node tree.local > 0 then
        {
          node_id  = tree.local.node_id;
          tree_nb  = node.tree_nb;
          priority = node.priority;
          distance = node.distance;
          parent   = Some node.node_id;
          rank     = node.rank
        }
        else tree.local;
      peers =
        match List.find_opt (fun peer -> peer.node_id = node.node_id) tree.peers with
        | None -> node :: tree.peers
        | Some _ -> List.map (fun peer -> if peer.node_id = node.node_id then node else peer) tree.peers
    }

  let delete_node tree node =
    let new_peers = List.filter (fun peer -> (peer.node_id <> node)) tree.peers in
    {
      peers = new_peers;
      local = match tree.local.parent with
        | None -> tree.local
        | Some id -> if id = node then 
          begin
            let rec max_list l = match l with
            | [] -> invalid_arg "empty list"
            | x :: [] -> x
            | x :: remain -> max x (max_list remain) in
              max_list new_peers
          end 
          else tree.local;
          (* TODO : more to do if dead parent was also root *)
    }

  let is_stable tree =
    List.for_all (fun peer -> peer.priority = tree.local.priority) tree.peers

  let get_parent tree =
    match tree.local.parent with
    | None -> None
    | Some parent ->
    List.find_opt (fun peer -> peer.node_id = parent) tree.peers

  let get_childs tree =
    List.filter (fun peer -> 
      match peer.parent with
      | None -> false
      | Some parent -> parent = tree.local.node_id) tree.peers

  let get_broken_links tree =
    tree.peers
    |> List.filter (fun (peer:Node.t) -> 
        match peer.parent with
        | None -> false
        | Some parent -> parent <> tree.local.node_id)
    |> List.filter (fun (peer:Node.t) -> 
        match get_parent tree with
        | None -> true
        | Some parent -> parent.node_id <> peer.node_id)

  let print tree =
    printf "   Local : %s\n%!" (Sexplib.Sexp.to_string (Node.sexp_of_t tree.local));
    printf "      Parent      : %s\n%!"
      (match get_parent tree with
      | None -> "none"
      | Some parent -> (Sexplib.Sexp.to_string (Node.sexp_of_t (parent))));
    List.iter (fun peer -> printf "      Children    : %s\n%!" (Sexplib.Sexp.to_string (Node.sexp_of_t peer))) (get_childs tree);
    List.iter (fun peer -> printf "      Broken link : %s\n%!" (Sexplib.Sexp.to_string (Node.sexp_of_t peer))) (get_broken_links tree)
end

module Set = struct
  
  type t = tree list

  module type S = sig
    val create : t
    val is_stable : t -> bool
    val parents : t -> Node.t list
    val min_dist : t -> int
    val next_tree : 'a list -> int
    val update_tree_set : t -> Node.t -> t
    val delete_node : t -> Node.id -> t
    val print : t -> unit
  end

  module Configure(Conf : Configuration) : S = struct
    module Tree = Configure(Conf)

    let create =
      [{
        local = 
        {
          node_id  = Conf.local_id;
          tree_nb  = 0;
          priority = (Conf.local_prio, Conf.local_id);
          distance = 0;
          parent   = None;
          rank     = 0 
        };
        peers = []
      }]

    let is_stable tree_set =
      List.for_all (fun x -> Tree.is_stable x) tree_set

    let parents tree_set = 
      List.map (fun tree -> Tree.get_parent tree) tree_set 
      |> Common.Option.flatten 
      |> Common.Option.get
      |> List.sort_uniq (compare) 

    let min_dist tree_set =
      (List.fold_left (fun a b -> if a.local.distance < b.local.distance then a else b) (List.hd tree_set) tree_set).local.distance

    let next_tree tree_set =
      List.length tree_set

    let update_tree_set tree_set node =
      let open Node in
      let tree_set = match List.exists (fun tree -> tree.local.tree_nb = node.tree_nb) tree_set with
      | true -> tree_set
      | false ->
        {
          local =
            {
              node_id  = Conf.local_id;
              tree_nb  = node.tree_nb;
              distance = 0;
              parent   = None;
              rank     = 0;
              priority = match (min_dist tree_set > Conf.max_dist) with
              | true -> (Conf.local_prio, Conf.local_id)
              | false -> (0, Conf.local_id) (*TODO *)
            };
          peers = [];
        } :: tree_set in
      let tree_set =
        List.map (fun tree -> 
          if tree.local.tree_nb = node.tree_nb then Tree.update tree node else tree)
          tree_set in
      let tree_set = match is_stable tree_set && List.length tree_set < Conf.max_trees with
      | false -> tree_set
      | true -> match min_dist tree_set > Conf.max_dist with
        | false -> tree_set
        | true ->
          {
            local =
              {
                node_id  = Conf.local_id;
                tree_nb  = next_tree tree_set;
                distance = 0;
                parent   = None;
                rank     = 0;
                priority = (Conf.local_prio, Conf.local_id)
              };
            peers = [];
          } :: tree_set in
      tree_set

    let delete_node tree_set node =
      List.map (fun x -> Tree.delete_node x node) tree_set

    let print tree_set =
      tree_set
      |> List.sort (fun a b -> compare a.local.tree_nb b.local.tree_nb)
      |> List.iter (fun tree -> printf "Tree nb %i:\n%!" tree.local.tree_nb; Tree.print tree)
  end
end
