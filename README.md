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
```
# swan-updown -h
swan-updown helps create ipsec interfaces

Usage: swan-updown [OPTIONS]

Options:
  -p, --prefix <prefix>  the prefix of the created interfaces, default to [swan]
  -n, --netns <netns>    Optional network namespace to move interfaces into
  -m, --master <master>  Optional master device to assign interfaces to
      --to-stdout        send log to stdout, otherwise the log will be sent to syslog
  -d, --debug...         set it multiple times to increase log level, [0: Error, 1: Warn, 2: Info, 3: Debug]
  -h, --help             Print help
  -V, --version          Print version
```
#### reminder
By default `swan-updown` uses `syslog`, if you want it to use `env_logger`, please specify `--to-stdout`.

### what it will do
#### interface
It will [create / destroy] XFRM interface when an SA is [established / deleted].

The name of the interface will be `{prefix}{hex encoded if_id}`.
The `prefix` can be specified by `--prefix` argument and the `if_id` is the `PLUTO_IF_ID_IN` environment variable.

`swan-updown` also adds altnames to the interface. The altnames will show
- the local and remote IKEIDs pair
- the local and remote IP addresses pair

Additionally, if `--netns` is specified, the created interface will be moved into the given netns.

