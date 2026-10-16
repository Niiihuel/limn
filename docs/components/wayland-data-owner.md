# WaylandDataOwner

An explicit Linux/Wayland DnD owner for several surfaces on one connection.
The gallery page is a static lifecycle illustration: no native owner is
attached, and no files are received. The canonical API and safety contract
are in [File drops](../file-drop.md#shared-owner-and-surface-endpoints).

## Import

```rust
use limn_dnd::{EndpointStatus, FileDropEndpoint, ReceiveStatus, WaylandDataOwner};
```

## Usage

Attach a display owner, register real surfaces from that same connection,
and poll each endpoint. These unsafe operations require the native lifetimes
and coordinated host reader described in the canonical guide. Do not attach
alongside stock SDK clipboard ownership; this DnD-only slice does not supply
a replacement clipboard backend.

`endpoint.detach()` requests local retirement. Continue host dispatch until
Detached before releasing that native surface. `owner.close()` requests
connection cleanup; keep the display and host reader until owner Closed.
These checks express different lifetime boundaries:

```rust
use limn_dnd::{EndpointStatus, FileDropEndpoint, ReceiveStatus, WaylandDataOwner};

fn may_release_surface(endpoint: &FileDropEndpoint) -> bool {
    matches!(endpoint.status(), EndpointStatus::Detached(_))
}

fn may_stop_host(owner: &WaylandDataOwner) -> bool {
    matches!(owner.status(), ReceiveStatus::Closed(_))
}
```

Detaching A leaves B usable. An old terminal stays drainable after detach;
its acknowledgement cannot release B's active drop. Local Copy finish success
does not establish compositor receipt, clipboard functionality or an upload.
There are no interactive controls or keyboard actions on this gallery page.
