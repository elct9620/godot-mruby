# frozen_string_literal: true

require_relative "support/bench"

desc "Time a generated project's Ruby as it grows (BENCH_SIZES, BENCH_JSON)"
task :bench do
  Bench.report!(ENV)
end
