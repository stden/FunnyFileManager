<!-- Purpose: Index of user and developer tutorials. -->

# Filesystems, AWS, and S3

These pages describe what FunnyFileManager does today. They are not a general
Amazon Web Services course.

| Page | What it covers |
|---|---|
| [Local filesystems](filesystems.md) | Drives, panel listings, hidden names, links, and copies that ask before overwriting |
| [AWS profiles](aws.md) | The credentials file, the profile list, and which region the client uses |
| [S3](s3.md) | Object keys versus folders, the path syntax, library operations, and the MCP tools |

Read them in that order. The local filesystem is what both panels show. AWS is
only how a profile is named and authenticated. S3 is a separate object store:
the library and the MCP server can list, copy, upload, download, and delete
objects. The panels do not browse a bucket yet. Choosing an S3 row in the
drive menu leaves the panel on its local directory and says so in the status
line.

## For developers

The [Rust best practices tutorial](rust-best-practices.md) covers Rust design,
testing, and verification using examples from FFM.
