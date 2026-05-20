#!/usr/bin/env ruby
# frozen_string_literal: true

require "open3"
require "tempfile"

ROOT = File.expand_path("..", __dir__)
SCRIPT = File.join(ROOT, "scripts", "check-rust-panics.rb")

def run_scanner(source)
  Tempfile.create(["check-rust-panics", ".rs"]) do |file|
    file.write(source)
    file.flush
    _stdout, stderr, status = Open3.capture3(SCRIPT, file.path)
    return [status.success?, stderr]
  end
end

def assert_passes(name, source)
  success, stderr = run_scanner(source)
  return if success

  warn "#{name} should have passed"
  warn stderr
  exit 1
end

def assert_fails(name, source, expected)
  success, stderr = run_scanner(source)
  if success
    warn "#{name} should have failed"
    exit 1
  end

  return if stderr.include?(expected)

  warn "#{name} failed without expected output"
  warn "expected: #{expected}"
  warn stderr
  exit 1
end

assert_fails(
  "unwrap in shipped code",
  "fn main() { let value = Some(1).unwrap(); }\n",
  "Some(1).unwrap()",
)

assert_fails(
  "expect in shipped code",
  "fn main() { let value = Some(1).expect(\"present\"); }\n",
  ".expect(\"present\")",
)

assert_fails(
  "panic in shipped code",
  "fn main() { panic!(\"bad\"); }\n",
  "panic!(\"bad\")",
)

assert_fails(
  "todo in shipped code",
  "fn main() { todo!(\"finish this\"); }\n",
  "todo!(\"finish this\")",
)

assert_fails(
  "unimplemented in shipped code",
  "fn main() { unimplemented!(\"later\"); }\n",
  "unimplemented!(\"later\")",
)

assert_fails(
  "dbg in shipped code",
  "fn main() { let value = dbg!(1); let _ = value; }\n",
  "dbg!(1)",
)

assert_passes(
  "test module panics are ignored",
  <<~RUST,
    fn production() -> Option<i32> { Some(1) }

    #[cfg(test)]
    mod tests {
        #[test]
        fn allows_test_panics() {
            let value = Some(1).unwrap();
            assert_eq!(value, 1);
            panic!("test-only panic");
            let value = dbg!(value);
            assert_eq!(value, 1);
            todo!("test-only placeholder");
        }
    }
  RUST
)

assert_passes(
  "safe production code",
  "fn main() -> Result<(), String> { Some(1).ok_or_else(|| \"missing\".to_string())?; Ok(()) }\n",
)
