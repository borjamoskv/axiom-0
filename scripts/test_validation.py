"""Regressions for validation coverage and subprocess failure propagation."""

import contextlib
import importlib.util
import io
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("axiom_validation", ROOT / "validate.py")
validation = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validation)


class ExecutionTests(unittest.TestCase):
    def run_captured(self, steps):
        output = io.StringIO()
        errors = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            status = validation.run_steps(steps)
        return status, output.getvalue(), errors.getvalue()

    def test_failed_core_or_firmware_stops_before_later_steps(self):
        for stage in ("core tests", "QEMU SMP and panic"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as directory:
                sentinel = Path(directory) / "unexpected later step"
                steps = [
                    (stage, [sys.executable, "-c", "raise SystemExit(17)"]),
                    (
                        "later step",
                        [
                            sys.executable,
                            "-c",
                            "from pathlib import Path; import sys; Path(sys.argv[1]).touch()",
                            str(sentinel),
                        ],
                    ),
                ]
                status, output, _ = self.run_captured(steps)
                self.assertNotEqual(status, 0)
                self.assertFalse(sentinel.exists())
                self.assertNotIn("VALIDATION PASS", output)

    def test_missing_executable_fails_without_success(self):
        with tempfile.TemporaryDirectory() as directory:
            missing = str(Path(directory) / "missing executable")
            status, output, _ = self.run_captured([("missing tool", [missing])])
        self.assertNotEqual(status, 0)
        self.assertNotIn("VALIDATION PASS", output)

    def test_success_runs_every_step_from_repository_root(self):
        with tempfile.TemporaryDirectory() as directory:
            records = [Path(directory) / name for name in ("first cwd", "second cwd")]
            steps = [
                (
                    f"step {index}",
                    [
                        sys.executable,
                        "-c",
                        "from pathlib import Path; import sys; "
                        "Path(sys.argv[1]).write_text(str(Path.cwd()))",
                        str(record),
                    ],
                )
                for index, record in enumerate(records)
            ]
            previous = Path.cwd()
            try:
                os.chdir(directory)
                status, output, _ = self.run_captured(steps)
                self.assertEqual(Path.cwd(), Path(directory).resolve())
            finally:
                os.chdir(previous)
            self.assertEqual(status, 0)
            self.assertEqual(output.count("VALIDATION PASS"), 1)
            for record in records:
                self.assertEqual(Path(record.read_text()).resolve(), ROOT)


class ContractTests(unittest.TestCase):
    def test_invalid_timeouts_launch_nothing(self):
        for timeout in ("0", "-1", "nan", "inf", "-inf"):
            with (
                self.subTest(timeout=timeout),
                mock.patch.object(validation.subprocess, "run") as process,
                mock.patch.object(validation, "run_steps") as run_steps,
                contextlib.redirect_stderr(io.StringIO()),
                self.assertRaises(SystemExit) as error,
            ):
                validation.main([f"--timeout={timeout}"])
            self.assertNotEqual(error.exception.code, 0)
            process.assert_not_called()
            run_steps.assert_not_called()

    def test_plan_retains_hosted_bare_metal_and_guest_contracts(self):
        qemu = "/tool path/qemu-system-aarch64"
        commands = [list(command) for _, command in validation.validation_steps(qemu, 17.5)]
        for command in (
            ["cargo", "check", "--workspace", "-q"],
            ["cargo", "test", "--workspace", "-q"],
            ["cargo", "clippy", "--workspace", "--", "-D", "warnings"],
        ):
            with self.subTest(command=command):
                self.assertIn(command, commands)
        self.assertTrue(
            any(
                command[:2] == ["cargo", "test"]
                and "--workspace" in command
                and "--no-default-features" in command
                for command in commands
            ),
            "host tests must also exercise the library without default features",
        )
        self.assertTrue(
            any(
                command[:2] == ["cargo", "build"]
                and {"--lib", "--release", "--no-default-features", "--target", "aarch64-unknown-none"}
                <= set(command)
                for command in commands
            ),
            "host tests cannot replace the bare-metal library build",
        )
        firmware_clippy = [
            command
            for command in commands
            if command[:2] == ["cargo", "clippy"] and "--manifest-path" in command
        ]
        self.assertTrue(firmware_clippy, "the standalone firmware needs its own lint check")
        self.assertTrue(any("--all-features" in command for command in firmware_clippy))
        for command in firmware_clippy:
            self.assertEqual(command[command.index("--target") + 1], "aarch64-unknown-none")
            self.assertEqual(command[-3:], ["--", "-D", "warnings"])
        guest_commands = [
            command
            for command in commands
            if any(Path(argument).name == "run.py" for argument in command)
        ]
        self.assertTrue(any("--smp" not in command for command in guest_commands))
        self.assertTrue(
            any("--smp" in command and "--check-panic" in command for command in guest_commands)
        )
        for command in guest_commands:
            self.assertEqual(command[command.index("--qemu") + 1], qemu)
            self.assertEqual(float(command[command.index("--timeout") + 1]), 17.5)


if __name__ == "__main__":
    unittest.main()
