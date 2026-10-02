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

      module_function

      def verify!(project)
        measures = SIZES.map { |files| measure(project, files) }
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
        limit = ALLOWANCE * SIZES.last / SIZES.first
        return if large <= small * limit

        raise "#{what} took #{large} µs for #{SIZES.last} files against #{small} µs for #{SIZES.first}, " \
              "more than #{limit} times as long"
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
