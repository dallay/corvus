---
title: GPG Key Setup for Maven Central Publishing
---

This guide explains how to configure GPG keys for publishing artifacts to Maven Central via GitHub Actions.

## Prerequisites

- GPG installed: `brew install gnupg` (macOS) or `apt install gnupg` (Linux)
- Sonatype account at https://central.sonatype.com

---

## Step 1: Generate Your GPG Key Pair

```bash
# Generate a new GPG key
gpg --full-gen-key
```

**Recommended configuration:**
- **Key type:** RSA and RSA (default)
- **Key size:** `4096` bits (minimum required by Maven Central)
- **Validity:** `0` = no expiration (or your preferred duration)
- **Name:** Your real name
- **Email:** Your GitHub-associated email
- **Passphrase:** A strong password (store it securely)

---

## Step 2: Create a Signing Subkey (Recommended for CI/CD)

Using subkeys is more secure because they can be revoked independently from your master key.

```bash
# List your keys to find the key ID
gpg --list-secret-keys

# Output example:
# sec   rsa4096 2026-02-11 [SC]
#       ABCD1234EFGH5678IJKL9012MNOP3456QR
# uid           [ultimate] Your Name <your@email.com>
# ssb   rsa4096 2026-02-11 [E]

# Your key ID is the last 16 characters after "sec rsa4096"
# Example: ABCD1234EFGH5678IJKL9012MNOP3456QR
```

**Create a dedicated signing subkey:**

```bash
gpg --edit-key YOUR_KEY_ID

# At the GPG prompt:
gpg> addkey
# Select: (4) RSA (sign only)
# Size: 4096
# Duration: 2y (2 years, or your preference)
# Confirm: y
# Passphrase: enter or use same as master key

gpg> save
gpg> quit
```

---

## Step 3: Export Your ASCII Armored Private Key

**Option A: Subkey only (RECOMMENDED for CI/CD)**

```bash
# Export the PRIVATE KEY (subkey) in ASCII format
gpg --export-secret-keys --armor YOUR_SUBKEY_ID > private-subkey.asc

# Verify it starts with:
# -----BEGIN PGP PRIVATE KEY BLOCK-----

# Display the content (to copy):
cat private-subkey.asc

# Copy the ENTIRE content including BEGIN and END lines
```

**Option B: Full key (if you need the master key as well)**

```bash
gpg --export-secret-keys --armor YOUR_KEY_ID > private-key.asc
```

---

## Step 4: Get Your Subkey ID

```bash
# List secret keys to identify the signing subkey
gpg --list-secret-keys

# Output example:
# sec   rsa4096 2026-02-11 [SC]
#       ABCD1234EFGH5678IJKL9012MNOP3456QR
# uid           [ultimate] Your Name <your@email.com>
# ssb   rsa4096 2026-02-11 [E]    ← Encryption subkey
# ssb   rsa4096 2026-02-11 [S]    ← THIS is the signing subkey
#       1234567890ABCDEF1234567890ABCDEF12345678

# The subkey ID is the last 16 characters
# Example: 1234567890ABCDEF1234567890ABCDEF12345678
```

---

## Step 5: Publish Your Public Key to a Key Server

```bash
# Send to keyserver (choose one)
gpg --keyserver keyserver.ubuntu.com --send-keys YOUR_KEY_ID

# Alternative keyservers:
# gpg --keyserver keys.openpgp.org --send-keys YOUR_KEY_ID
# gpg --keyserver pgp.mit.edu --send-keys YOUR_KEY_ID
```

---

## Step 6: Configure GitHub Secrets

Go to **GitHub → Repository → Settings → Secrets and variables → Actions** and create these 4 secrets:

| Secret Name | Value |
|-------------|-------|
| `SIGNING_IN_MEMORY_KEY` | **ENTIRE** content of `private-subkey.asc` (including BEGIN and END lines) |
| `SIGNING_IN_MEMORY_KEY_PASSWORD` | The passphrase you set when creating the key |
| `MAVEN_CENTRAL_USERNAME` | Your Sonatype username |
| `MAVEN_CENTRAL_PASSWORD` | **User Token** from Sonatype (NOT your password) |

---

## Step 7: Create User Token in Sonatype

1. Go to https://central.sonatype.com
2. Login with your Sonatype account
3. Click on your avatar → **Profile**
4. Find **User Token** or **Generate Token**
5. Create a token and copy:
   - **Username** → `MAVEN_CENTRAL_USERNAME`
   - **Password** → `MAVEN_CENTRAL_PASSWORD`

**Important:** Never use a never-expiring token. User tokens should be rotated periodically.

---

## Step 8: Verify GPG Key Format

Ensure your `SIGNING_IN_MEMORY_KEY` secret contains:

```
-----BEGIN PGP PRIVATE KEY BLOCK-----

lQHYBF...  (base64 encoded key data)
...
-----END PGP PRIVATE KEY BLOCK-----
```

**NOT binary format** (which would look like random binary characters).

---

## Testing Locally

```bash
# Export variables and test publish
export ORG_GRADLE_PROJECT_signingInMemory_KEY="$(cat private-subkey.asc)"
export ORG_GRADLE_PROJECT_signingInMemory_KEY_PASSWORD="your_passphrase"
export ORG_GRADLE_PROJECT_mavenCentralUsername="your_sonatype_username"
export ORG_GRADLE_PROJECT_mavenCentralPassword="your_sonatype_token"

# Test (dry-run mode if available)
./gradlew publishToMavenCentral --dry-run

# Or actual publish
./gradlew publishToMavenCentral
```

---

## Troubleshooting

### "gpg: signing failed: Inappropriate ioctl for device"

This happens in CI environments without a TTY. The configuration in this project uses in-memory keys, which should work around this issue.

### "No secret key" error

Verify that:
1. The key is exported with `--armor` flag (ASCII format)
2. The secret includes `-----BEGIN PGP PRIVATE KEY BLOCK-----`
3. The correct key ID is used (if using subkey)

### Key expired

To extend expiration:
```bash
gpg --edit-key YOUR_KEY_ID
gpg> list
gpg> key 1        # Select the signing subkey (second key)
gpg> expire
# Set new expiration
gpg> save
gpg> quit

# Re-publish to keyserver
gpg --keyserver keyserver.ubuntu.com --send-keys YOUR_KEY_ID
```

---

## Security Best Practices

1. **NEVER commit** any `.asc` files or private keys to version control
2. **ALWAYS use User Tokens** in Sonatype, not your account password
3. **Subkeys expire** - set calendar reminders to renew them before expiration
4. **Backup your master key** securely (encrypted USB, hardware wallet, etc.)
5. **Consider using hardware security keys** (YubiKey) for maximum security
6. **Rotate tokens** periodically, especially if you suspect any compromise

---

## References

- [Maven Central: GPG Signing Requirements](https://central.sonatype.org/publish/requirements/gpg/)
- [GitHub: Use secrets in workflows](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-secrets)
- [GnuPG Documentation](https://gnupg.org/documentation/)


