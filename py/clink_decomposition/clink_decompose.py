#!/usr/bin/env python3
"""
CL8NK/CL9NK Decomposition System

Entry ---CL8|9NK---> LOGICAL CONNECTIVES ---DECOMPOSE---> MATHEMATICAL EXPRESSIONS

Usage:
    clink-decompose <entry> [--navigator cl8nk|cl9nk] [--discipline <discipline>] [--output <format>] [--output-dir <dir>]
"""

import argparse
import sys
import os
from datetime import datetime
from typing import Optional, List

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# Import from absolute paths
from clink_decomposition.core.input_processor import InputProcessor, NavigatorOutput
from clink_decomposition.core.context_selector import ContextSelector
from clink_decomposition.core.composition_engine import CompositionEngine
from clink_decomposition.output.generator import OutputGenerator, DecompositionOutput
from clink_decomposition.validation.validator import Validator
from clink_decomposition.registers import list_disciplines, get_discipline


def main():
    parser = argparse.ArgumentParser(
        description="CL8NK/CL9NK Decomposition: Entry --> LOGICAL CONNECTIVES --> MATHEMATICAL EXPRESSIONS",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  clink-decompose torque
  clink-decompose torque --navigator cl8nk --discipline topology
  clink-decompose "Feynman's_Sprinkler" --navigator cl9nk --discipline category_theory
  clink-decompose torque --all-disciplines
  clink-decompose --list-disciplines
  clink-decompose --list-entries
        """
    )
    
    parser.add_argument(
        'entry',
        nargs='?',
        help='Catalog entry to decompose (e.g., torque, "Feynman\'s_Sprinkler")'
    )
    
    parser.add_argument(
        '--navigator', '-n',
        choices=['cl8nk', 'cl9nk'],
        default='cl8nk',
        help='Navigator to use (default: cl8nk)'
    )
    
    parser.add_argument(
        '--discipline', '-d',
        default=None,
        help='Mathematical discipline (default: auto-selected)'
    )
    
    parser.add_argument(
        '--all-disciplines',
        action='store_true',
        help='Decompose using all available disciplines'
    )
    
    parser.add_argument(
        '--output', '-o',
        choices=['md', 'tex', 'lean', 'json', 'console'],
        default='console',
        help='Output format (default: console)'
    )
    
    parser.add_argument(
        '--output-dir',
        default='outputs',
        help='Directory for output files (default: outputs)'
    )
    
    parser.add_argument(
        '--list-disciplines',
        action='store_true',
        help='List available disciplines and exit'
    )
    
    parser.add_argument(
        '--validate',
        action='store_true',
        help='Run validation checks on output'
    )
    
    parser.add_argument(
        '--verbose', '-v',
        action='store_true',
        help='Verbose output'
    )
    
    args = parser.parse_args()
    
    # List disciplines and exit
    if args.list_disciplines:
        print("Available disciplines:")
        for d in list_disciplines():
            print(f"  - {d}")
        sys.exit(0)
    
    # Check if entry is provided
    if not args.entry:
        parser.print_help()
        sys.exit(1)
    
    # Initialize components
    processor = InputProcessor()
    selector = ContextSelector()
    engine = CompositionEngine()
    generator = OutputGenerator()
    validator = Validator()
    
    # Process the entry
    if args.verbose:
        print(f"Processing entry: {args.entry}")
        print(f"Navigator: {args.navigator}")
    
    try:
        # Step 1: Run through navigator
        if args.verbose:
            print("Step 1: Running navigator...")
        
        navigator_output = processor.process_entry(args.entry, args.navigator)
        
        if args.verbose:
            print(f"  CLINK Tuple: {navigator_output.clink_tuple}")
            print(f"  Primitives: {navigator_output.primitives}")
            print(f"  Promoted Atoms: {navigator_output.promoted_atoms}")
        
        # Determine disciplines to use
        if args.all_disciplines:
            disciplines = list_disciplines()
        else:
            # Select discipline
            if args.discipline:
                disciplines = [args.discipline]
            else:
                discipline = selector.select_discipline(
                    args.entry, 
                    args.navigator
                )
                disciplines = [discipline]
        
        # Validate disciplines
        available = list_disciplines()
        for d in disciplines:
            if d not in available:
                print(f"Error: Discipline '{d}' not available. Available: {available}", file=sys.stderr)
                sys.exit(1)
        
        # Step 2-4: Decompose for each discipline
        all_outputs = []
        for discipline in disciplines:
            if args.verbose:
                print(f"\nStep 2-4: Decomposing for discipline '{discipline}'...")
            
            # Get context
            context = selector.get_context_parameters(args.entry, discipline)
            
            # Map primitives
            register = get_discipline(discipline)
            if register is None:
                print(f"Error: Cannot load discipline '{discipline}'", file=sys.stderr)
                continue
            
            mappings = [
                register.map_primitive(p, context) 
                for p in navigator_output.primitives
            ]
            
            # Compose expression
            mathematical_expression = engine.compose(
                mappings, 
                discipline, 
                context
            )
            
            # Step 5: Generate output
            validation_results = {}
            if args.validate:
                if args.verbose:
                    print("  Validating...")
                validation_results = validator.validate(mathematical_expression, navigator_output)
            
            # Generate final output
            output = generator.generate(
                navigator_output,
                mathematical_expression,
                validation_results
            )
            
            # Update metadata
            output.metadata.update({
                "context": context,
                "discipline_specific": True
            })
            
            all_outputs.append(output)
            
            if args.verbose:
                print(f"  Conventional: {output.conventional_math[:100]}...")
                print(f"  Lean: {output.lean_code[:100]}...")
        
        # Output results
        if not all_outputs:
            print("No outputs generated.", file=sys.stderr)
            sys.exit(1)
        
        if args.output == 'console':
            for output in all_outputs:
                print_output(output)
        else:
            for output in all_outputs:
                filepath = generator.generate_file(
                    output,
                    args.output_dir,
                    args.output
                )
                print(f"Generated: {filepath}")
    
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        if args.verbose:
            import traceback
            traceback.print_exc()
        sys.exit(1)


def print_output(output: DecompositionOutput):
    """Print output to console"""
    print("=" * 80)
    print(f"CLINK Decomposition: {output.entry_name}")
    print(f"Navigator: {output.navigator} | Discipline: {output.discipline}")
    print(f"CLINK Tuple: {output.clink_tuple}")
    print("=" * 80)
    print()
    
    print("📊 STRUCTURAL INFORMATION")
    print("-" * 80)
    print(f"Promoted Atoms: {', '.join(output.promoted_atoms) if output.promoted_atoms else 'None'}")
    print(f"Tier: {output.tier or 'Unknown'}")
    print(f"Distance: {output.distance or 'Unknown'}")
    print()
    
    print("📐 CONVENTIONAL MATHEMATICAL EXPRESSION")
    print("-" * 80)
    print(output.conventional_math)
    print()
    
    print("🔷 LEAN CODE")
    print("-" * 80)
    print(output.lean_code)
    print()
    
    if output.validation_results:
        print("✅ VALIDATION RESULTS")
        print("-" * 80)
        for check, result in output.validation_results.items():
            status = "✅" if result.passed else "❌"
            print(f"{status} {check}: {result.message}")
        print()
    
    print("=" * 80)


if __name__ == '__main__':
    main()
