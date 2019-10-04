$.ajaxSetup({timeout:1000});
window.addEventListener("message", function (event) {if(event.data == "refresh"){refresh();}}, false);

var nodes = new vis.DataSet();
var edges = new vis.DataSet();
var container;
var data = {
    nodes: nodes,
    edges: edges
};
var default_node_color = {background:"#7BBCFF", border:"#00356D", 
                          highlight:{background:"#2397FF", border:"#00356D"}};
var default_node_text_color = "#000000";
var disabled_node_color = {background:"#F5F5F5", border:"#BBBBBB", 
                           highlight:{background:"#F5F5F5", border:"#BBBBBB"}};
var disabled_node_text_color = "#BBBBBB";
var default_edge_color = {color:"grey", highlight:"grey", hover:"grey"};
var disabled_edge_color = {color:"#EEEEEE", highlight:"#EEEEEE", hover:"#EEEEEE"};
var flow_edge_color = {color:"#1387FF", highlight:"#1387FF", hover:"#1387FF"};

var options = {
    edges:{font:{face:'courier', size:10, multi:"html", bold:{face:'courier bold', size:12}}, selectionWidth:0},
    nodes:{shape:"box", margin:4, //heightConstraint:50, widthConstraint:92,  
           font:{face:'courier', size:10, multi:"html", align:'left', bold:{face:'courier bold', size:12}},
           color:default_node_color},
    physics:{enabled:true}
};
var network;
var routereditor;
var pluginseditor;

redraw();

$(document).ready( function() {
    container = document.getElementById('graph')
    $('#tab-container').easytabs();
    routereditor = new JSONEditor(document.getElementById('router'), {mode: 'view'});
    pluginseditor = new JSONEditor(document.getElementById('plugins'), {mode: 'view'});
});

function parentLinkId(from, to, tree_nb) {
    return "" + tree_nb + "_" + from + "_" + to;
}

function brokenLinkId(node1, node2) {
    return "bk_" + (node1>node2?node1:node2) + "_" + (node1>node2?node2:node1);
}

function updateDataSet(set, elements) {
    set.update(elements);
    set.getIds().forEach(function(id) {
        if(elements.find(function(elem){return elem.id === id}) === undefined) {
            try{ set.remove(id); } catch(err) {}
        }
    });
}

function containsLinkBetween(edgeset, node1, node2) {
    return edgeset.find(function (edge) {
            return (edge.from === node1 && edge.to === node2) ||
                (edge.from === node2 && edge.to === node1);
        }) !== undefined ;
}

function updateNodes(zServices, plugins) {
    zNodes = Object.keys(zServices).map( function(id, idx) {
        z_addr = zServices[id].locators[0].split('/').pop().split(':');
        z_inet_addr = z_addr.shift();
        z_port = z_addr.shift();
        http_str = "";
        http_json = plugins["/@/" + zServices[id].pid + "/plugins/zenoh-http"];
        if(http_json) {
            http_port = http_json.locators[0].split(':').pop();
            http_str = "\n" +
                "_______________" + "\n" +
                "http:" + http_port;
        }
        yaks_str = "";
        yaks_json = plugins["/@/" + zServices[id].pid + "/plugins/yaks"];
        if(yaks_json) {
            yaks_str = "\n" +
                "_______________" + "\n" +
                "YAKS";
        }
        label = "<b>" + zServices[id].hostname.substring(0, 12) + "</b>\n" + 
                z_inet_addr + "\n" + "tcp:" + z_port + http_str + yaks_str + "\n" +
                "_______________";
        return {id: zServices[id].pid, label: label};
    });
    updateDataSet(nodes, zNodes);
}

function getSessionForPeer(service, peerid) {
    peer = service.trees.peers.find(function(peer){return (peer.pid === peerid);});
    return service.sessions.find(function(session){return (session.sid === peer.sid);});
}

function updateEdges(zServices, plugins) {
    treelinks = Object.keys(zServices).map( function(id, idx) {
        return zServices[id].trees.tree_set
            .filter( function (tree){ return tree.local.parent != null;})
            .map( function (tree){ 
                tpup = getSessionForPeer(zServices[id], tree.local.parent).stats.out_msgs_tp;
                tpdwn = getSessionForPeer(zServices["/@/" + tree.local.parent], zServices[id].pid).stats.out_msgs_tp;
                label= "<b>" + (tpup + tpdwn).toString() + " m/s" + "</b>";
                if ((tpup + tpdwn) > 0)
                {
                    arrows = '';
                    if (tpup > 0) {arrows += 'to, '}
                    if (tpdwn > 0) {arrows += 'from, '}
                    return {
                    id: parentLinkId(zServices[id].pid, tree.local.parent),
                    from: zServices[id].pid, to: tree.local.parent, 
                    label:label, arrows: arrows, 
                    color:flow_edge_color, dashes:false, width:4};
                }
                else
                {
                    return {
                    id: parentLinkId(zServices[id].pid, tree.local.parent),
                    from: zServices[id].pid, to: tree.local.parent, 
                    label:label, arrows:'', 
                    color:null, dashes:false, width:2};
                }
            });
        }).flat();

    brokenLinks = Object.keys(zServices).map( function(id, idx) {
        return zServices[id].trees.peers
            .filter( function (peer){ return !containsLinkBetween(treelinks, zServices[id].pid, peer.pid);})
            .map( function (peer){ 
                return {
                    id: brokenLinkId(zServices[id].pid, peer.pid),
                    from: zServices[id].pid, to: peer.pid, 
                    label:"<b></b>", arrows:'', 
                    color:null, dashes:true, width:1};
            });
        }).flat();
    links = treelinks.concat(brokenLinks);

    updateDataSet(edges, links);
}

function failure(){
    $("#message").html("Unable to contact server!");
    edges.forEach(edge => {
        edge.label = "<b></b>";
        edge.arrows = "";
        edge.color = null;
        if(edge.width > 2){edge.width = 2;};
        edges.update(edge);
    });
    nodes.forEach(node => {
        node.color = null;
        if(node.shape == 'image'){node.image = node.image.replace(/00DD00/g, "F5F5F5").replace(/000000/g, "BBBBBB")}
        nodes.update(node);
    });
    options.nodes.color = disabled_node_color;
    options.edges.color = disabled_edge_color;
    options.nodes.font.color = disabled_node_text_color;
    options.nodes.font.bold.color = disabled_node_text_color;
    network.setOptions(options);
    routereditor.updateText("{}");
    pluginseditor.updateText("{}");
}

function cleanfailure(){
    $("#message").html("");
    options.nodes.color = default_node_color;
    options.edges.color = default_edge_color;
    options.nodes.font.color = default_node_text_color;
    options.nodes.font.bold.color = default_node_text_color;
    network.setOptions(options);
}

function update(zServices, plugins) {
    cleanfailure();
    updateNodes(zServices, plugins);
    updateEdges(zServices, plugins);
    showDetails(zServices, plugins);
}

function refresh() {
    $.getJSON("/@/*", zServices => {
        $.getJSON("/@/*/plugins/*", plugins => {
            update(zServices, plugins);
        }).fail(failure);
    }).fail(failure);
}

function autorefresh() {
    $("#autorefresh").toggleClass("loading");
    if($("#autorefresh").hasClass("loading"))
    {
        function periodicupdate(){
            $.getJSON("/@/*", zServices => {
                $.getJSON("/@/*/plugins/*", plugins => {
                    update(zServices, plugins);
                    if($("#autorefresh").hasClass("loading"))
                    {
                        setTimeout(periodicupdate, 500);
                    }
                })
                .fail(() => {
                    failure();
                    if($("#autorefresh").hasClass("loading"))
                    {
                        setTimeout(periodicupdate, 500);
                    }
                });
            })
            .fail(() => {
                failure();
                if($("#autorefresh").hasClass("loading"))
                {
                    setTimeout(periodicupdate, 500);
                }
            });
        }
        periodicupdate();
    }
}

function mapkeys(dict, fun) {
    return Object.keys(dict).reduce(
        (accu, current) => {accu[fun(current)] = dict[current]; return accu;}, 
        {});
}

function showDetails(zServices, plugins) {
    if(network && network.getSelectedNodes()[0]) {
        nodeid = ""+network.getSelectedNodes()[0]
        if(zServices && zServices["/@/" + nodeid]) {
            routereditor.update(zServices["/@/" + nodeid]);
            pluginseditor.update(mapkeys(plugins, (key) => key.split('/').pop()));
        } else {
            $.getJSON("/@/" + network.getSelectedNodes()[0], zServices => {
                $.getJSON("/@/" + network.getSelectedNodes()[0] + "/plugins/*", plugins => {
                    pluginseditor.update(mapkeys(plugins, key => key.split('/').pop()));
                });
                routereditor.update(zServices["/@/" + nodeid]);
            })
            .fail(failure);
        }
    } else {
        routereditor.updateText("{}");
        pluginseditor.updateText("{}");
    }
}

function resetgraph(){
    network = new vis.Network(container, data, options);
    network.on("click", showDetails);
    network.on("dragStart", showDetails);
}

function redraw() {
    $.getJSON("/@/*", zServices => {
        $.getJSON("/@/*/plugins/*", plugins => {
            resetgraph();
            update(zServices, plugins);
        })
        .fail(() => {
            resetgraph();
            failure();
        });
    })
    .fail(() => {
        resetgraph();
        failure();
    });
}