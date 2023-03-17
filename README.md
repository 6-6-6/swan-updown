## swan-updown

see [updown plugin](https://docs.strongswan.org/docs/5.9/plugins/updown.html).

First, it parses `PLUTO_*` and cli args.

Then it helps create ipsec interfaces on demand and log to syslog.


### usage
To utilize `swan-updown`, specify
```
connections.<conn>.children.<child>.updown = swan-updown [OPTIONS]
```
in `swanctl.conf`

For its arguments, see `swan-updown -h`.

### what it will do
#### interface
It will [create / destroy] XFRM interface when an SA is [established / deleted].

The name of the interface will be `{prefix}{hex encoded if_id}`.
The `prefix` can be specified by `--prefix` argument and the `if_id` is the `PLUTO_IF_ID_IN` environment variable.

`swan-updown` also adds altnames to the interface. The altnames will show
- the local and remote IKEIDs pair
- the local and remote IP addresses pair

Additionally, if `--netns` is specified, the created interface will be moved into the given netns.

