# AGP generates precise manifest/resource entry-point rules for the Application,
# activities, receivers and services. Library reflection/JNI requirements belong
# to their bundled consumer rules. Blent has no name-based reflective class lookup
# requiring a package-wide keep. Add only specific proven entry-point rules here,
# with permanent coverage; keep optimization and missing-class diagnostics enabled.
