"""Print what an AT-SPI client (a screen reader) is given by every application on the bus."""
import sys
import gi
gi.require_version('Atspi', '2.0')
from gi.repository import Atspi

def states(node):
    names = []
    s = node.get_state_set()
    for flag, name in [(Atspi.StateType.FOCUSABLE, 'focusable'), (Atspi.StateType.FOCUSED, 'focused'),
                       (Atspi.StateType.ENABLED, 'enabled'), (Atspi.StateType.CHECKED, 'checked'),
                       (Atspi.StateType.SELECTED, 'selected'), (Atspi.StateType.EXPANDED, 'expanded'),
                       (Atspi.StateType.EXPANDABLE, 'expandable'), (Atspi.StateType.PRESSED, 'pressed')]:
        if s.contains(flag):
            names.append(name)
    return ','.join(names)

def walk(node, depth, out):
    try:
        role, name, desc = node.get_role_name(), node.get_name() or '', node.get_description() or ''
        extra = ''
        try:
            v = node.get_value_iface()
            if v is not None:
                extra += f' value={v.get_current_value():.2f}'
        except Exception:
            pass
        try:
            a = node.get_action_iface()
            if a is not None:
                extra += ' actions=' + '/'.join(a.get_action_name(i) for i in range(a.get_n_actions()))
        except Exception:
            pass
        out.append(f"{'  ' * depth}{role} | {name!r} | {desc!r} | {states(node)}{extra}")
        for i in range(node.get_child_count()):
            walk(node.get_child_at_index(i), depth + 1, out)
    except Exception as error:
        out.append(f"{'  ' * depth}<error {error}>")

desktop = Atspi.get_desktop(0)
out = []
for i in range(desktop.get_child_count()):
    walk(desktop.get_child_at_index(i), 0, out)
print('\n'.join(out) if out else 'NO APPLICATIONS ON THE ACCESSIBILITY BUS')
