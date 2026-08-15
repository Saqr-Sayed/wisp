import Gio from 'gi://Gio';
import Shell from 'gi://Shell';

const IFACE = `
<node>
  <interface name="com.saqr.wisp.WindowSource">
    <method name="GetActive">
      <arg type="s" name="app_id" direction="out"/>
      <arg type="s" name="title" direction="out"/>
    </method>
  </interface>
</node>`;

let dbusId = null;
let exported = null;
let tracker = null;

function getActive() {
    const win = global.display.get_focus_window();
    if (!win)
        return ['', ''];
    const app = tracker?.get_window_app(win);
    const appId = app?.get_id() ?? win.get_wm_class() ?? '';
    const title = win.get_title() ?? '';
    return [appId, title];
}

export default class WispWindowSource {
    enable() {
        const impl = {
            GetActive() {
                return getActive();
            },
        };
        exported = Gio.DBusExportedObject.wrapJSObject(IFACE, impl);
        exported.export(Gio.DBus.session, '/com/saqr/wisp/WindowSource');
        dbusId = Gio.DBus.session.own_name(
            'com.saqr.wisp.WindowSource',
            Gio.BusNameOwnerFlags.NONE,
            null,
            null);
        tracker = Shell.WindowTracker.get_default();
    }

    disable() {
        if (dbusId) {
            Gio.DBus.session.unown_name(dbusId);
            dbusId = null;
        }
        if (exported) {
            exported.unexport();
            exported = null;
        }
        tracker = null;
    }
}
