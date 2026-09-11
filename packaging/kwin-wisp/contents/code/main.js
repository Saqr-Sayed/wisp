// kwin-wisp: push active-window changes to the Wisp daemon over D-Bus.
var SERVICE = "com.saqr.wisp.WindowSource";
var PATH = "/com/saqr/wisp/WindowSource";
var IFACE = "com.saqr.wisp.WindowSource";

function pushActive(client) {
    var app = "";
    var title = "";
    try {
        if (client) {
            title = String(client.caption || "");
            // resourceClass is "instance class" or a single class; take the class part.
            var rc = String(client.resourceClass || "");
            var parts = rc.split(" ");
            app = parts.length > 1 ? parts[1] : (parts[0] || "");
        }
        callDBus(SERVICE, PATH, IFACE, "PushActive", app, title);
    } catch (e) {}
}

if (workspace.windowActivated) {
    workspace.windowActivated.connect(pushActive);
} else if (workspace.clientActivated) {
    workspace.clientActivated.connect(pushActive);
}
// Seed current state at (re)load.
try {
    pushActive(workspace.activeWindow || workspace.activeClient || null);
} catch (e) {}
