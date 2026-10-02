# frozen_string_literal: true

require_relative "support/bench"

desc "Time a game's Ruby beside GDScript (BENCH_SIZES, BENCH_JSON, BENCH_BASELINE)"
task :bench do
  Bench.report!(ENV)
end
