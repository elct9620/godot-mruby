# frozen_string_literal: true

require "tmpdir"

require_relative "../bench"

module Godot
  module Loader
    # Plays the bench scene in a generated project and in one twenty times its
    # size, and holds what Ruby timed to growing with the files: a cost paid
    # per file that itself grows with the project, such as walking every
    # directory for each file, multiplies past the allowance. Part of
    # Loader.verify!.
    module Scaling
      SIZES = [20, 400].freeze
      # How far past the projects' size ratio the larger may go, for the
      # fixed costs and noise of a small run.
      ALLOWANCE = 2
      # What Ruby timed that has to stay proportional to the files.
      TIMED = %w[run_by_name attach].freeze
      # How many times the projects are measured while one stays past the
      # allowance, so a run slowed once by a loaded machine is not read as
      # growth; each size keeps its fastest.
      ATTEMPTS = 3

      module_function

      def verify!(project)
        measures = measure_fastest(project)
        verify_reached!(measures)
        verify_attached!(measures)
      end

      # @behavior RL-037
      def verify_reached!(measures)
        verify_proportional!("Reaching every file by name", measures.map { |measure| measure.fetch("run_by_name") })
      end

      # @behavior RL-038
      def verify_attached!(measures)
        verify_proportional!("Giving nodes their scripts", measures.map { |measure| measure.fetch("attach") })
      end

      def verify_proportional!(what, (small, large))
        return if proportional?(small, large)

        raise "#{what} took #{large} µs for #{SIZES.last} files against #{small} µs for #{SIZES.first}, " \
              "more than #{limit} times as long"
      end

      # Each size's measures, measured again while any stays past the
      # allowance, up to ATTEMPTS times, each keeping the fastest it took.
      def measure_fastest(project)
        measures = measure_sizes(project)
        (ATTEMPTS - 1).times do
          break if all_proportional?(measures)

          measures = keep_fastest(measures, measure_sizes(project))
        end
        measures
      end

      def measure_sizes(project)
        SIZES.map { |files| measure(project, files) }
      end

      def all_proportional?(measures)
        TIMED.all? { |name| proportional?(*measures.map { |measure| measure.fetch(name) }) }
      end

      # Each size's measures, each the faster of `kept` and `again`.
      def keep_fastest(kept, again)
        kept.zip(again).map { |one, other| one.merge(other) { |_, first, second| [first, second].min } }
      end

      def proportional?(small, large)
        large <= small * limit
      end

      # How many times as long the larger project may take.
      def limit
        ALLOWANCE * SIZES.last / SIZES.first
      end

      # What Ruby timed in a generated project of `files` library files, with
      # the addon of `source`.
      def measure(source, files)
        Dir.mktmpdir do |project|
          Bench::Project.generate(project, files, source)
          Bench.play(project)
        end
      end
    end
  end
end
