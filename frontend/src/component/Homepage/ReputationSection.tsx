"use client";

import { Card, CardContent } from "@/components/ui/card";
import { Shield, Activity } from "lucide-react";
import { motion } from "framer-motion";

export default function ReputationSection() {
  const items = [
    {
      icon: <Shield className="w-6 h-6 text-white" />,
      title: "Earn Credibility",
      description: "Your actions speak louder than words. On-chain reputation means you can't fake your track record."
    },
    {
      icon: <Activity className="w-6 h-6 text-white" />,
      title: "Zero Manipulation",
      description: "Smart contracts handle all logic, ensuring no central party can alter outcomes or rewards."
    }
  ];

  return (
    <section className="relative py-24 overflow-hidden">
      {/* Background Glow */}
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[500px] bg-orange-500/10 blur-[150px] -z-10 rounded-full animate-pulse"></div>
      
      <div className="relative z-10 max-w-7xl mx-auto px-6">
        <div className="relative bg-white/[0.02] backdrop-blur-3xl border border-white/[0.05] rounded-[2.5rem] p-12 lg:p-20 overflow-hidden">
          {/* Subtle decorative background pattern */}
          <div className="absolute inset-0 opacity-[0.03] pointer-events-none bg-[radial-gradient(circle_at_2px_2px,_white_1px,_transparent_0)] bg-[size:40px_40px]"></div>

          <motion.div 
            className="text-center space-y-4 mb-20 relative z-20"
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
            viewport={{ once: true }}
          >
            <h2 className="text-4xl lg:text-5xl font-bold text-white leading-tight">
              Where Insight Becomes Reputation
            </h2>
            <p className="text-gray-300 text-lg max-w-2xl mx-auto">
              Every prediction is recorded on-chain. Build a verifiable track record that proves your expertise.
            </p>
          </motion.div>

          <div className="grid md:grid-cols-2 gap-8 max-w-7xl mx-auto relative z-20">
            {items.map((item, index) => (
              <motion.div 
                key={index}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.6, delay: index * 0.1 }}
                viewport={{ once: true }}
              >
                <Card className="group bg-gray-950/60 backdrop-blur-xl border border-white/10 hover:border-orange-500/50 transition-all duration-300 hover:-translate-y-1 h-full shadow-2xl">
                  <CardContent className="p-8 space-y-6">
                    <div className="w-14 h-14 bg-gradient-to-r from-orange-500 to-yellow-500 rounded-xl flex items-center justify-center group-hover:scale-110 transition-transform duration-300 shadow-[0_0_20px_rgba(249,115,22,0.3)]">
                      {item.icon}
                    </div>
                    <div className="space-y-3">
                      <h3 className="text-2xl font-bold text-white">{item.title}</h3>
                      <p className="text-gray-300 text-lg leading-relaxed">{item.description}</p>
                    </div>
                  </CardContent>
                </Card>
              </motion.div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
