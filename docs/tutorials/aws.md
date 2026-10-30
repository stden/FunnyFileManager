<!-- Purpose: Explain which AWS profile and region data FFM actually loads. -->

# AWS profiles

FFM uses AWS for one thing: credentials and a region for S3. It does not manage
IAM, EC2, or the AWS console.

## The credentials file

Profiles are section names in the shared credentials file:

| Host | Path |
|---|---|
| Linux and macOS | `$HOME/.aws/credentials` |
| Windows | `%USERPROFILE%\.aws\credentials` |

```ini
[default]
aws_access_key_id = <access-key-id>
aws_secret_access_key = <secret-access-key>

[photos]
aws_access_key_id = <access-key-id>
aws_secret_access_key = <secret-access-key>
```

Put real keys only in that file. Do not copy them into the repository, a
panel, or a chat.

`list_aws_profiles` keeps each `[section]` name, in file order. It does not
read `~/.aws/config`, and the region field on each profile value stays empty.
If `default` is missing, or the file cannot be read, `default` is inserted
at the front anyway. A missing file is an empty profile list plus that
inserted name. It is not an error.

## What the client authenticates with

Both the TUI library (`src/s3.rs`, feature `s3`) and the MCP server build a
client the same way:

1. Credentials come from `ProfileFileCredentialsProvider` for the profile
   name you asked for. That is the shared credentials file. The client does
   not switch over to environment variables, a container role, or an EC2
   instance role.
2. The region is the SDK's default region provider. If that provider has no
   region, the client uses `us-east-1`.

A profile that exists only as a heading, with no keys, still appears in the
list. The first S3 call then fails when the provider cannot sign.

## Where the profile list shows up

The TUI drive menu includes one `S3: name` row per profile only when the
binary was built with `--features s3`. A normal `cargo build` does not enable
`s3`. The MCP server links the S3 SDK itself, so its `s3_list_profiles` tool
does not need that feature flag.

Selecting the row in `Alt+F1` or `Alt+F2` does not open the account. The
status line says the profile was selected and that the S3 browser is not
available yet. The panel stays on its local directory. The next page is what
you can do with the profile after that.
