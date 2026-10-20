# T1160 — explicit backup exclusions

Retain dataExtractionRules. The minSDK27 UnusedAttribute advisory describes an
attribute consumed on API31+, where the packaged data-extraction-rules contains
both cloud-backup and device-transfer exclusions for the actual blent.xml
preference store. Older platforms use fullBackupContent with the same explicit
exclusion. allowBackup=false remains an additional policy; removing newer rules
would discard the explicit OEM device-transfer protection documented by T257.

Identity/trust files remain under AndroidConnectionFactory.directory's
noBackupFilesDir/blent-connection; identity signing remains AndroidKeyStore-owned.
No source or schema change, suppression, credential relocation or transfer occurs.
Retained BackupRulesTest passes both SDK27 and SDK34 offline (2 tests, no skips),
parsing the packaged manifest references and every configured transport's token
exclusion. These are packaged-policy checks, not a claim that every OEM transfer
implementation has been tested. Source hashes, native JUnit and build log follow.
