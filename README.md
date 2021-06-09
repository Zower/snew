# Snew
An experimental Rust wrapper for the Reddit API. It is not ready for use. See the issues page for information on missing features.

# Usage
See the documentation. You can build it yourself in the regular manner. Clone the repo, then:
```shell
cargo doc --open
```

Unit tests are not included in this repo, as they have to include secret and password.

# User agent 
Recommended format for the user_agent
```
<platform>:<app ID>:<version string> (by /u/<reddit username>)
```