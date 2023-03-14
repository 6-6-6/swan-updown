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

The name of the interface will be based on the `--prefix` argument and the `PLUTO_IF_ID_IN` environment variable.

Additionally, if `--netns` is specified, the interface will be moved into the given netns.

