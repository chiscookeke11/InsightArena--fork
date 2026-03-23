"use client";

import { motion } from "framer-motion";

const leftItems = [
  "High Fees",
  "Centralized Resolution",
  "Custodial Funds",
  "Opaque Data",
  "Hidden Manipulation",
];

const rightItems = [
  "Low Protocol Fees",
  "Decentralized Oracle Resolution",
  "Non-Custodial Smart Contracts",
  "Transparent On-Chain Data",
  "Provably Fair Outcomes",
];

const XIcon = () => (
  <span
    style={{
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      width: 22,
      height: 22,
      borderRadius: "50%",
      background: "#e53e3e",
      flexShrink: 0,
    }}
  >
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <path
        d="M2 2L8 8M8 2L2 8"
        stroke="#101828"
        strokeWidth="3"
        strokeLinecap="round"
      />
    </svg>
  </span>
);

const CheckIcon = () => (
  <span
    style={{
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      width: 22,
      height: 22,
      borderRadius: "50%",
      background: "#38a169",
      flexShrink: 0,
    }}
  >
    <svg width="11" height="9" viewBox="0 0 11 9" fill="none">
      <path
        d="M1 4.5L4 7.5L10 1"
        stroke="#101828"
        strokeWidth="3"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  </span>
);

export default function ComparisonSection() {
  return (
    <section className="relative py-16 md:py-24 px-6" style={{ background: "none" }}>
      <div className="max-w-6xl mx-auto">
        {/* Section Title */}
        <motion.h2
          className="text-3xl sm:text-4xl md:text-5xl font-bold text-white text-center mb-10 md:mb-16"
          initial={{ opacity: 0, y: 16 }}
          whileInView={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          viewport={{ once: true }}
        >
          <span className="text-transparent bg-clip-text bg-gradient-to-r from-orange-400 to-yellow-400">
            More
          </span>{" "}Than a Prediction App
        </motion.h2>

        {/* Comparison Cards */}
        <div
          className="comparison-grid grid grid-cols-1 md:grid-cols-2 gap-6 md:gap-12"
        >
          {/* Left Card — Traditional */}
          <motion.div
            initial={{ opacity: 0, x: -24 }}
            whileInView={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.5, delay: 0.1 }}
            viewport={{ once: true }}
            className="bg-gray-950/60 border border-[#1e2d45] rounded-xl p-7"
          >
            <h3 className="text-lg sm:text-xl md:text-2xl font-bold text-white mb-4 md:mb-6">
              Traditional Prediction Platforms
            </h3>
            <ul className="flex flex-col gap-4 font-medium text-gray-300">
              {leftItems.map((item, i) => (
                <motion.li
                  key={i}
                  initial={{ opacity: 0, x: -12 }}
                  whileInView={{ opacity: 1, x: 0 }}
                  transition={{ duration: 0.35, delay: 0.15 + i * 0.07 }}
                  viewport={{ once: true }}
                  className="flex text-gray-400 items-center gap-2 text-sm sm:text-base md:text-lg"
                >
                  <XIcon />
                  {item}
                </motion.li>
              ))}
            </ul>
          </motion.div>

          {/* Right Card — InsightArena */}
          <motion.div
            initial={{ opacity: 0, x: 24 }}
            whileInView={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.5, delay: 0.1 }}
            viewport={{ once: true }}
            className="bg-gray-950/60 border border-[#1e2d45] rounded-xl p-7"
          >
            <h3 className="text-lg sm:text-xl md:text-2xl font-bold text-white mb-4 md:mb-6">
              InsightArena
            </h3>
            <ul className="flex flex-col gap-4 font-medium text-gray-300">
              {rightItems.map((item, i) => (
                <motion.li
                  key={i}
                  initial={{ opacity: 0, x: 12 }}
                  whileInView={{ opacity: 1, x: 0 }}
                  transition={{ duration: 0.35, delay: 0.15 + i * 0.07 }}
                  viewport={{ once: true }}
                  className="flex text-gray-400 items-center gap-2 text-sm sm:text-base md:text-lg"
                >
                  <CheckIcon />
                  {item}
                </motion.li>
              ))}
            </ul>
          </motion.div>
        </div>
      </div>

      <style>{`
        @media (max-width: 640px) {
          .comparison-grid {
            grid-template-columns: 1fr !important;
          }
        }
      `}</style>
    </section>
  );
}
